use super::super::*;
use crate::mood::inference::{
    self, extract_content, parse_mood_intensity_response, LlamaRuntimeIntent, LlamaServer,
    LlamaServerStartOptions,
};
use crate::types::BaseMood;
use base64::Engine;
use image::GenericImageView;
use serde::Serialize;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};

const GREEN: &str = "\x1b[32m";
const RED: &str = "\x1b[31m";
const YELLOW: &str = "\x1b[33m";
const CYAN: &str = "\x1b[36m";
const DIM: &str = "\x1b[2m";
const BOLD: &str = "\x1b[1m";
const RESET: &str = "\x1b[0m";

const MOOD_GRAMMAR: &str = r#"root ::= mood " " intensity
mood ::= "epic" | "tension" | "sadness" | "comedy" | "romance" | "horror" | "peaceful" | "mystery"
intensity ::= "1" | "2" | "3""#;

#[derive(Clone)]
struct PageEntry {
    rel_path: String,
    full_path: PathBuf,
    raw_bytes: Vec<u8>,
    mood: String,
    intensity: u8,
    confidence: f64,
}

#[derive(Clone, Debug)]
struct EncodedImage {
    mime: &'static str,
    b64: String,
}

#[derive(Clone, Copy, Debug, Serialize)]
enum ImagePrepMode {
    Legacy672Jpeg,
    CenterPng1024,
    AllPng1024,
}

impl ImagePrepMode {
    fn as_str(self) -> &'static str {
        match self {
            Self::Legacy672Jpeg => "legacy_672_jpeg",
            Self::CenterPng1024 => "center_png_1024",
            Self::AllPng1024 => "all_png_1024",
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize)]
enum PromptStyle {
    SequenceWindow,
    WideSequenceWindow,
    CenterFocus,
    NarrativeFocus,
    WideNarrativeFocus,
}

impl PromptStyle {
    fn as_str(self) -> &'static str {
        match self {
            Self::SequenceWindow => "sequence_window",
            Self::WideSequenceWindow => "wide_sequence_window",
            Self::CenterFocus => "center_focus",
            Self::NarrativeFocus => "narrative_focus",
            Self::WideNarrativeFocus => "wide_narrative_focus",
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum ImageRole {
    Context,
    Center,
}

#[derive(Clone, Copy, Debug, Serialize)]
enum ContextWindow {
    Triad,
    Wide5,
}

impl ContextWindow {
    fn as_str(self) -> &'static str {
        match self {
            Self::Triad => "triad",
            Self::Wide5 => "wide5",
        }
    }

    fn indices(self, center: usize, total: usize) -> Vec<usize> {
        let clamp = |idx: isize| -> usize {
            idx.clamp(0, total.saturating_sub(1) as isize) as usize
        };
        match self {
            Self::Triad => vec![
                clamp(center as isize - 1),
                center,
                clamp(center as isize + 1),
            ],
            Self::Wide5 => vec![
                clamp(center as isize - 2),
                clamp(center as isize - 1),
                center,
                clamp(center as isize + 1),
                clamp(center as isize + 2),
            ],
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize)]
enum DecisionStrategy {
    Majority,
    Viterbi,
    CenterOverride,
    Direct,
    Wide5SelectiveRepair,
    FocusedReprompt,
    OcrReprompt,
    SemanticReprompt,
}

impl DecisionStrategy {
    fn as_str(self) -> &'static str {
        match self {
            Self::Majority => "majority",
            Self::Viterbi => "viterbi",
            Self::CenterOverride => "center_override",
            Self::Direct => "direct",
            Self::Wide5SelectiveRepair => "wide5_selective_repair",
            Self::FocusedReprompt => "focused_reprompt",
            Self::OcrReprompt => "ocr_reprompt",
            Self::SemanticReprompt => "semantic_reprompt",
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize)]
struct SamplingConfig {
    name: &'static str,
    temperature: f32,
    top_p: Option<f32>,
    top_k: Option<u32>,
    seed: Option<u32>,
    max_tokens: u32,
}

#[derive(Clone, Copy, Debug, Serialize)]
struct RuntimeProfile {
    name: &'static str,
    #[serde(skip_serializing)]
    intent: LlamaRuntimeIntent,
    context_size: u32,
    parallel_slots: u32,
}

#[derive(Clone, Debug)]
struct ModelCandidate {
    key: &'static str,
    label: &'static str,
    model_path: PathBuf,
    mmproj_path: PathBuf,
    reasoning_format: Option<&'static str>,
}

#[derive(Clone)]
struct LiveExperiment {
    model_key: &'static str,
    prep: ImagePrepMode,
    context: ContextWindow,
    prompt_style: PromptStyle,
    sampling: SamplingConfig,
    runtime: RuntimeProfile,
    grammar: bool,
    decision: DecisionStrategy,
}

#[derive(Clone)]
struct DerivedExperiment {
    input_ids: &'static [&'static str],
}

#[derive(Clone)]
enum ExperimentMode {
    Cached { cache_filename: &'static str },
    Live(LiveExperiment),
    Derived(DerivedExperiment),
}

#[derive(Clone)]
struct ExperimentSpec {
    id: &'static str,
    label: &'static str,
    notes: &'static str,
    mode: ExperimentMode,
}

#[derive(Clone, Debug)]
struct WindowPrediction {
    members: Vec<usize>,
    left: usize,
    center: usize,
    right: usize,
    mood: String,
    intensity: u8,
    elapsed_s: f64,
}

#[derive(Clone, Debug)]
struct PageVoteStats {
    counts: [u32; 8],
    weighted: [f64; 8],
    intensity_sum: [u32; 8],
    intensity_count: [u32; 8],
    center_vote: Option<BaseMood>,
    center_intensity: Option<u8>,
    winner_score: f64,
    runner_up_score: f64,
}

#[derive(Clone, Debug)]
struct PagePrediction {
    mood: String,
    intensity: u8,
    winner_score: f64,
    runner_up_score: f64,
    note: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize)]
struct MoodBreakdown {
    total: u32,
    strict: u32,
    relaxed: u32,
}

#[derive(Clone, Debug, Serialize)]
struct ExperimentSummary {
    id: String,
    label: String,
    source: String,
    status: String,
    model: String,
    prep: String,
    decision: String,
    sampling: String,
    runtime: String,
    strict: u32,
    relaxed: u32,
    intensity: u32,
    total: u32,
    errors: u32,
    second_pass_pages: u32,
    avg_window_s: f64,
    min_window_s: f64,
    max_window_s: f64,
    vram_mib: Option<u32>,
    cache_path: Option<String>,
    notes: String,
    per_mood: BTreeMap<String, MoodBreakdown>,
}

pub async fn run() {
    inference::lower_current_process_priority();

    let (chapters, skipped, page_limit) = load_realtest_dataset();
    let total_pages: usize = chapters.values().map(|pages| pages.len()).sum();
    assert!(!chapters.is_empty(), "No RealTest chapters selected");

    let realtest_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("manga-mood-ai/test-images/RealTest");
    let results_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("manga-mood-ai/results");
    let _ = std::fs::create_dir_all(&results_dir);

    let filter: Option<Vec<String>> = std::env::var("REALTEST_FILTER")
        .ok()
        .map(|f| f.split(',').map(|s| s.trim().to_string()).collect());
    let verbose = std::env::var("REALTEST_VERBOSE")
        .ok()
        .map(|v| matches!(v.as_str(), "1" | "true" | "yes"))
        .unwrap_or(false);

    println!("\n  {BOLD}RealTest Benchmark Suite{RESET}\n");
    if let Some(ref filters) = filter {
        println!("  Filter: {:?}", filters);
    }
    if let Some(limit) = page_limit {
        println!("  {DIM}Page limit per chapter: {limit}{RESET}");
    }
    println!(
        "  Found: {} chapters, {} pages ({} skipped)\n",
        chapters.len(),
        total_pages,
        skipped
    );
    println!(
        "  {DIM}Default mode now runs the comparison suite. Use REALTEST_EXPERIMENTS=id1,id2 to narrow it or REALTEST_PAGE_LIMIT=N for a smoke run.{RESET}\n"
    );

    let models_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("manga-mood-ai/models");
    let models = discover_models(&models_dir);
    let experiment_filter = parse_experiment_filter();

    let mut summaries = Vec::new();
    let suite = build_suite();
    let baseline_id = "baseline_cache";

    for spec in suite {
        if !experiment_filter.is_empty() && !experiment_filter.contains(&spec.id) {
            continue;
        }

        match &spec.mode {
            ExperimentMode::Cached { cache_filename } => {
                let cache_path = results_dir.join(cache_filename);
                let summary = load_cached_summary(&spec, &chapters, &cache_path);
                print_experiment_summary(&summary);
                summaries.push(summary);
            }
            ExperimentMode::Live(live) => {
                let summary = run_live_experiment(
                    &spec,
                    live,
                    &models,
                    &chapters,
                    &results_dir,
                    verbose,
                )
                .await;
                print_experiment_summary(&summary);
                summaries.push(summary);
            }
            ExperimentMode::Derived(derived) => {
                let summary =
                    run_derived_experiment(&spec, derived, &chapters, &results_dir, &summaries);
                print_experiment_summary(&summary);
                summaries.push(summary);
            }
        }
    }

    assert!(
        !summaries.is_empty(),
        "No experiments selected. REALTEST_EXPERIMENTS filter removed everything."
    );

    let baseline = summaries
        .iter()
        .find(|s| s.id == baseline_id)
        .cloned()
        .unwrap_or_else(|| summaries[0].clone());

    print_comparison_table(&summaries, &baseline);
    save_suite_summary(&results_dir, &realtest_dir, &summaries, &baseline);
}

fn load_realtest_dataset() -> (BTreeMap<String, Vec<PageEntry>>, u32, Option<usize>) {
    let realtest_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("manga-mood-ai/test-images/RealTest");
    assert!(
        realtest_dir.exists(),
        "RealTest not found: {:?}",
        realtest_dir
    );

    let gt_path = realtest_dir.join("ground_truth.json");
    let gt_str = std::fs::read_to_string(&gt_path).expect("Failed to read ground_truth.json");
    let gt: HashMap<String, serde_json::Value> =
        serde_json::from_str(&gt_str).expect("Failed to parse ground_truth.json");

    let filter: Option<Vec<String>> = std::env::var("REALTEST_FILTER")
        .ok()
        .map(|f| f.split(',').map(|s| s.trim().to_string()).collect());
    let page_limit = std::env::var("REALTEST_PAGE_LIMIT")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .filter(|&v| v > 0);

    let mut chapters: BTreeMap<String, Vec<PageEntry>> = BTreeMap::new();
    let mut skipped = 0u32;

    for (rel_path, entry) in &gt {
        let parts: Vec<&str> = rel_path.split('/').collect();
        if parts.len() != 3 {
            continue;
        }

        let chapter_key = format!("{}/{}", parts[0], parts[1]);
        if let Some(ref filters) = filter {
            if !filters.iter().any(|f| {
                chapter_key == *f
                    || chapter_key.starts_with(&format!("{}/", f))
                    || f == &chapter_key.split('/').next().unwrap_or("")
            }) {
                continue;
            }
        }

        let full_path = realtest_dir.join(rel_path);
        if !full_path.exists() {
            skipped += 1;
            continue;
        }

        let raw_bytes = std::fs::read(&full_path)
            .unwrap_or_else(|e| panic!("Failed to read {}: {}", rel_path, e));
        let mood = entry["mood"].as_str().unwrap().to_string();
        let intensity = entry["intensity"].as_u64().unwrap() as u8;
        let confidence = entry["confidence"].as_f64().unwrap();
        chapters.entry(chapter_key).or_default().push(PageEntry {
            rel_path: rel_path.clone(),
            full_path,
            raw_bytes,
            mood,
            intensity,
            confidence,
        });
    }

    for pages in chapters.values_mut() {
        pages.sort_by_key(|page| {
            page.rel_path
                .split('/')
                .last()
                .unwrap()
                .split('.')
                .next()
                .unwrap()
                .parse::<u32>()
                .unwrap_or(0)
        });
        if let Some(limit) = page_limit {
            pages.truncate(limit);
        }
    }

    chapters.retain(|_, pages| !pages.is_empty());
    (chapters, skipped, page_limit)
}

fn build_suite() -> Vec<ExperimentSpec> {
    const GREEDY_HIST: SamplingConfig = SamplingConfig {
        name: "greedy_hist",
        temperature: 0.0,
        top_p: None,
        top_k: None,
        seed: None,
        max_tokens: 8192,
    };
    const OFFICIAL_THINK: SamplingConfig = SamplingConfig {
        name: "official_think",
        temperature: 1.0,
        top_p: Some(0.95),
        top_k: Some(20),
        seed: Some(42),
        max_tokens: 512,
    };
    const RUNTIME_DEFAULT: RuntimeProfile = RuntimeProfile {
        name: "primary_12288x1",
        intent: LlamaRuntimeIntent::BenchmarkPrimary,
        context_size: 12288,
        parallel_slots: 1,
    };
    const RUNTIME_QUIET: RuntimeProfile = RuntimeProfile {
        name: "quiet_8192x1",
        intent: LlamaRuntimeIntent::BenchmarkPrimary,
        context_size: 8192,
        parallel_slots: 1,
    };
    const RUNTIME_RESEARCH: RuntimeProfile = RuntimeProfile {
        name: "research_32768x4",
        intent: LlamaRuntimeIntent::ResearchLarge,
        context_size: 32768,
        parallel_slots: 4,
    };

    vec![
        ExperimentSpec {
            id: "baseline_cache",
            label: "Current baseline cache",
            notes: "Historical BL/1 winner loaded from cache, no rerun.",
            mode: ExperimentMode::Cached {
                cache_filename: "realtest_v12_thinking4b.json",
            },
        },
        ExperimentSpec {
            id: "t4b_official",
            label: "Qwen3-VL-4B-Thinking official sampling",
            notes: "Historical prompt, legacy preprocessing, majority vote.",
            mode: ExperimentMode::Live(LiveExperiment {
                model_key: "thinking4b",
                prep: ImagePrepMode::Legacy672Jpeg,
                context: ContextWindow::Triad,
                prompt_style: PromptStyle::SequenceWindow,
                sampling: OFFICIAL_THINK,
                runtime: RUNTIME_DEFAULT,
                grammar: false,
                decision: DecisionStrategy::Majority,
            }),
        },
        ExperimentSpec {
            id: "t4b_hist_live",
            label: "Qwen3-VL-4B-Thinking historical live",
            notes: "Historical prompt, legacy preprocessing, greedy decoding, majority vote.",
            mode: ExperimentMode::Live(LiveExperiment {
                model_key: "thinking4b",
                prep: ImagePrepMode::Legacy672Jpeg,
                context: ContextWindow::Triad,
                prompt_style: PromptStyle::SequenceWindow,
                sampling: GREEDY_HIST,
                runtime: RUNTIME_RESEARCH,
                grammar: false,
                decision: DecisionStrategy::Majority,
            }),
        },
        ExperimentSpec {
            id: "t4b_hist_center_override",
            label: "Qwen3-VL-4B-Thinking historical + center override",
            notes: "Historical prompt, legacy preprocessing, center-focused override on fragile pages.",
            mode: ExperimentMode::Live(LiveExperiment {
                model_key: "thinking4b",
                prep: ImagePrepMode::Legacy672Jpeg,
                context: ContextWindow::Triad,
                prompt_style: PromptStyle::SequenceWindow,
                sampling: GREEDY_HIST,
                runtime: RUNTIME_RESEARCH,
                grammar: false,
                decision: DecisionStrategy::CenterOverride,
            }),
        },
        ExperimentSpec {
            id: "t4b_focus_direct",
            label: "Qwen3-VL-4B-Thinking direct focus",
            notes: "Classify the current page only, with left/right pages used purely as context.",
            mode: ExperimentMode::Live(LiveExperiment {
                model_key: "thinking4b",
                prep: ImagePrepMode::CenterPng1024,
                context: ContextWindow::Triad,
                prompt_style: PromptStyle::CenterFocus,
                sampling: SamplingConfig {
                    name: "greedy_focus",
                    temperature: 0.0,
                    top_p: None,
                    top_k: None,
                    seed: None,
                    max_tokens: 512,
                },
                runtime: RUNTIME_DEFAULT,
                grammar: false,
                decision: DecisionStrategy::Direct,
            }),
        },
        ExperimentSpec {
            id: "t4b_narrative_focus",
            label: "Qwen3-VL-4B-Thinking narrative focus",
            notes: "Current-page prompt centered on narrative function to reduce epic overprediction.",
            mode: ExperimentMode::Live(LiveExperiment {
                model_key: "thinking4b",
                prep: ImagePrepMode::CenterPng1024,
                context: ContextWindow::Triad,
                prompt_style: PromptStyle::NarrativeFocus,
                sampling: SamplingConfig {
                    name: "greedy_narrative_focus",
                    temperature: 0.0,
                    top_p: None,
                    top_k: None,
                    seed: None,
                    max_tokens: 512,
                },
                runtime: RUNTIME_DEFAULT,
                grammar: false,
                decision: DecisionStrategy::Direct,
            }),
        },
        ExperimentSpec {
            id: "t4b_focus_short",
            label: "Qwen3-VL-4B-Thinking direct focus short output",
            notes: "Same 3-page center-focus prompt, but with a much shorter forced answer budget.",
            mode: ExperimentMode::Live(LiveExperiment {
                model_key: "thinking4b",
                prep: ImagePrepMode::CenterPng1024,
                context: ContextWindow::Triad,
                prompt_style: PromptStyle::CenterFocus,
                sampling: SamplingConfig {
                    name: "greedy_focus_short",
                    temperature: 0.0,
                    top_p: None,
                    top_k: None,
                    seed: None,
                    max_tokens: 32,
                },
                runtime: RUNTIME_DEFAULT,
                grammar: false,
                decision: DecisionStrategy::Direct,
            }),
        },
        ExperimentSpec {
            id: "t4b_focus_grammar_short",
            label: "Qwen3-VL-4B-Thinking direct focus short + grammar",
            notes: "Same 3-page center-focus prompt with short budget and strict mood/intensity grammar.",
            mode: ExperimentMode::Live(LiveExperiment {
                model_key: "thinking4b",
                prep: ImagePrepMode::CenterPng1024,
                context: ContextWindow::Triad,
                prompt_style: PromptStyle::CenterFocus,
                sampling: SamplingConfig {
                    name: "greedy_focus_short_grammar",
                    temperature: 0.0,
                    top_p: None,
                    top_k: None,
                    seed: None,
                    max_tokens: 16,
                },
                runtime: RUNTIME_DEFAULT,
                grammar: true,
                decision: DecisionStrategy::Direct,
            }),
        },
        ExperimentSpec {
            id: "t4b_wide5_narrative",
            label: "Qwen3-VL-4B-Thinking wide-5 narrative focus",
            notes: "Classify the current page using two previous and two next pages as narrative context.",
            mode: ExperimentMode::Live(LiveExperiment {
                model_key: "thinking4b",
                prep: ImagePrepMode::CenterPng1024,
                context: ContextWindow::Wide5,
                prompt_style: PromptStyle::WideNarrativeFocus,
                sampling: SamplingConfig {
                    name: "greedy_wide5_narrative",
                    temperature: 0.0,
                    top_p: None,
                    top_k: None,
                    seed: None,
                    max_tokens: 48,
                },
                runtime: RUNTIME_DEFAULT,
                grammar: false,
                decision: DecisionStrategy::Direct,
            }),
        },
        ExperimentSpec {
            id: "t4b_wide5_sequence",
            label: "Qwen3-VL-4B-Thinking wide-5 sequence vote",
            notes: "Historical sequence framing extended to 5 images, then overlapping-window majority vote.",
            mode: ExperimentMode::Live(LiveExperiment {
                model_key: "thinking4b",
                prep: ImagePrepMode::Legacy672Jpeg,
                context: ContextWindow::Wide5,
                prompt_style: PromptStyle::WideSequenceWindow,
                sampling: GREEDY_HIST,
                runtime: RUNTIME_DEFAULT,
                grammar: false,
                decision: DecisionStrategy::Majority,
            }),
        },
        ExperimentSpec {
            id: "t4b_wide5_sequence_viterbi",
            label: "Qwen3-VL-4B-Thinking wide-5 sequence + Viterbi",
            notes: "Historical sequence framing extended to 5 images, then transition-aware decoding.",
            mode: ExperimentMode::Live(LiveExperiment {
                model_key: "thinking4b",
                prep: ImagePrepMode::Legacy672Jpeg,
                context: ContextWindow::Wide5,
                prompt_style: PromptStyle::WideSequenceWindow,
                sampling: GREEDY_HIST,
                runtime: RUNTIME_DEFAULT,
                grammar: false,
                decision: DecisionStrategy::Viterbi,
            }),
        },
        ExperimentSpec {
            id: "t4b_wide5_selective",
            label: "Qwen3-VL-4B-Thinking wide-5 selective repair",
            notes: "Wide-5 sequence backbone with selective low-cost repairs on sadness/comedy/tension edge cases.",
            mode: ExperimentMode::Live(LiveExperiment {
                model_key: "thinking4b",
                prep: ImagePrepMode::Legacy672Jpeg,
                context: ContextWindow::Wide5,
                prompt_style: PromptStyle::WideSequenceWindow,
                sampling: GREEDY_HIST,
                runtime: RUNTIME_DEFAULT,
                grammar: false,
                decision: DecisionStrategy::Wide5SelectiveRepair,
            }),
        },
        ExperimentSpec {
            id: "t4b_wide5_narrative_grammar",
            label: "Qwen3-VL-4B-Thinking wide-5 narrative + grammar",
            notes: "Wide 5-page current-page prompt with strict short-form grammar.",
            mode: ExperimentMode::Live(LiveExperiment {
                model_key: "thinking4b",
                prep: ImagePrepMode::CenterPng1024,
                context: ContextWindow::Wide5,
                prompt_style: PromptStyle::WideNarrativeFocus,
                sampling: SamplingConfig {
                    name: "greedy_wide5_narrative_grammar",
                    temperature: 0.0,
                    top_p: None,
                    top_k: None,
                    seed: None,
                    max_tokens: 16,
                },
                runtime: RUNTIME_DEFAULT,
                grammar: true,
                decision: DecisionStrategy::Direct,
            }),
        },
        ExperimentSpec {
            id: "t4b_official_grammar",
            label: "Qwen3-VL-4B-Thinking official sampling + grammar",
            notes: "Historical prompt, legacy preprocessing, grammar-constrained output.",
            mode: ExperimentMode::Live(LiveExperiment {
                model_key: "thinking4b",
                prep: ImagePrepMode::Legacy672Jpeg,
                context: ContextWindow::Triad,
                prompt_style: PromptStyle::SequenceWindow,
                sampling: OFFICIAL_THINK,
                runtime: RUNTIME_DEFAULT,
                grammar: true,
                decision: DecisionStrategy::Majority,
            }),
        },
        ExperimentSpec {
            id: "t4b_center_png",
            label: "Qwen3-VL-4B-Thinking center PNG 1024",
            notes: "Historical prompt, high-res center page, majority vote.",
            mode: ExperimentMode::Live(LiveExperiment {
                model_key: "thinking4b",
                prep: ImagePrepMode::CenterPng1024,
                context: ContextWindow::Triad,
                prompt_style: PromptStyle::SequenceWindow,
                sampling: OFFICIAL_THINK,
                runtime: RUNTIME_DEFAULT,
                grammar: true,
                decision: DecisionStrategy::Majority,
            }),
        },
        ExperimentSpec {
            id: "t4b_all_png",
            label: "Qwen3-VL-4B-Thinking all pages PNG 1024",
            notes: "Historical prompt, all pages higher-res, majority vote.",
            mode: ExperimentMode::Live(LiveExperiment {
                model_key: "thinking4b",
                prep: ImagePrepMode::AllPng1024,
                context: ContextWindow::Triad,
                prompt_style: PromptStyle::SequenceWindow,
                sampling: OFFICIAL_THINK,
                runtime: RUNTIME_DEFAULT,
                grammar: true,
                decision: DecisionStrategy::Majority,
            }),
        },
        ExperimentSpec {
            id: "t4b_viterbi",
            label: "Qwen3-VL-4B-Thinking + Viterbi",
            notes: "Historical prompt, center PNG 1024, transition-aware sequence decoding.",
            mode: ExperimentMode::Live(LiveExperiment {
                model_key: "thinking4b",
                prep: ImagePrepMode::CenterPng1024,
                context: ContextWindow::Triad,
                prompt_style: PromptStyle::SequenceWindow,
                sampling: OFFICIAL_THINK,
                runtime: RUNTIME_DEFAULT,
                grammar: true,
                decision: DecisionStrategy::Viterbi,
            }),
        },
        ExperimentSpec {
            id: "t4b_focus",
            label: "Qwen3-VL-4B-Thinking + focused reprompt",
            notes: "Viterbi base, reprompt ambiguous pages while focusing on current page.",
            mode: ExperimentMode::Live(LiveExperiment {
                model_key: "thinking4b",
                prep: ImagePrepMode::CenterPng1024,
                context: ContextWindow::Triad,
                prompt_style: PromptStyle::SequenceWindow,
                sampling: OFFICIAL_THINK,
                runtime: RUNTIME_DEFAULT,
                grammar: true,
                decision: DecisionStrategy::FocusedReprompt,
            }),
        },
        ExperimentSpec {
            id: "t4b_ocr",
            label: "Qwen3-VL-4B-Thinking + OCR reprompt",
            notes: "Viterbi base, explicit text extraction on ambiguous pages.",
            mode: ExperimentMode::Live(LiveExperiment {
                model_key: "thinking4b",
                prep: ImagePrepMode::CenterPng1024,
                context: ContextWindow::Triad,
                prompt_style: PromptStyle::SequenceWindow,
                sampling: OFFICIAL_THINK,
                runtime: RUNTIME_DEFAULT,
                grammar: true,
                decision: DecisionStrategy::OcrReprompt,
            }),
        },
        ExperimentSpec {
            id: "t4b_semantic",
            label: "Qwen3-VL-4B-Thinking + semantic axes",
            notes: "Viterbi base, semantic-axis reprompt on ambiguous pages.",
            mode: ExperimentMode::Live(LiveExperiment {
                model_key: "thinking4b",
                prep: ImagePrepMode::CenterPng1024,
                context: ContextWindow::Triad,
                prompt_style: PromptStyle::SequenceWindow,
                sampling: OFFICIAL_THINK,
                runtime: RUNTIME_DEFAULT,
                grammar: true,
                decision: DecisionStrategy::SemanticReprompt,
            }),
        },
        ExperimentSpec {
            id: "t4b_quiet_viterbi",
            label: "Qwen3-VL-4B-Thinking quiet runtime",
            notes: "Production-friendly runtime, center PNG 1024, Viterbi decoding.",
            mode: ExperimentMode::Live(LiveExperiment {
                model_key: "thinking4b",
                prep: ImagePrepMode::CenterPng1024,
                context: ContextWindow::Triad,
                prompt_style: PromptStyle::SequenceWindow,
                sampling: OFFICIAL_THINK,
                runtime: RUNTIME_QUIET,
                grammar: true,
                decision: DecisionStrategy::Viterbi,
            }),
        },
        ExperimentSpec {
            id: "q35_4b_viterbi",
            label: "Qwen3.5-4B + Viterbi",
            notes: "Historical prompt, center PNG 1024, official sampling.",
            mode: ExperimentMode::Live(LiveExperiment {
                model_key: "qwen35_4b",
                prep: ImagePrepMode::CenterPng1024,
                context: ContextWindow::Triad,
                prompt_style: PromptStyle::SequenceWindow,
                sampling: OFFICIAL_THINK,
                runtime: RUNTIME_DEFAULT,
                grammar: true,
                decision: DecisionStrategy::Viterbi,
            }),
        },
        ExperimentSpec {
            id: "q35_9b_viterbi",
            label: "Qwen3.5-9B + Viterbi",
            notes: "Historical prompt, center PNG 1024, official sampling.",
            mode: ExperimentMode::Live(LiveExperiment {
                model_key: "qwen35_9b",
                prep: ImagePrepMode::CenterPng1024,
                context: ContextWindow::Triad,
                prompt_style: PromptStyle::SequenceWindow,
                sampling: OFFICIAL_THINK,
                runtime: RUNTIME_DEFAULT,
                grammar: true,
                decision: DecisionStrategy::Viterbi,
            }),
        },
        ExperimentSpec {
            id: "q3vl_2b_viterbi",
            label: "Qwen3-VL-2B-Instruct + Viterbi",
            notes: "Historical prompt, center PNG 1024, official-ish sampling.",
            mode: ExperimentMode::Live(LiveExperiment {
                model_key: "qwen3vl_2b",
                prep: ImagePrepMode::CenterPng1024,
                context: ContextWindow::Triad,
                prompt_style: PromptStyle::SequenceWindow,
                sampling: SamplingConfig {
                    name: "official_like",
                    ..GREEDY_HIST
                },
                runtime: RUNTIME_DEFAULT,
                grammar: true,
                decision: DecisionStrategy::Viterbi,
            }),
        },
        ExperimentSpec {
            id: "q3vl_2b_wide5_narrative",
            label: "Qwen3-VL-2B-Instruct wide-5 narrative focus",
            notes: "Cheap 2B current-page classifier with two pages of past/future context.",
            mode: ExperimentMode::Live(LiveExperiment {
                model_key: "qwen3vl_2b",
                prep: ImagePrepMode::CenterPng1024,
                context: ContextWindow::Wide5,
                prompt_style: PromptStyle::WideNarrativeFocus,
                sampling: SamplingConfig {
                    name: "greedy_wide5_narrative_2b",
                    temperature: 0.0,
                    top_p: None,
                    top_k: None,
                    seed: None,
                    max_tokens: 32,
                },
                runtime: RUNTIME_DEFAULT,
                grammar: false,
                decision: DecisionStrategy::Direct,
            }),
        },
        ExperimentSpec {
            id: "t4b_ensemble_viterbi",
            label: "Thinking4B + focus ensemble",
            notes: "Weighted multi-signal fusion: hist live + focus + Qwen3-VL-2B + Qwen3.5-9B, then Viterbi.",
            mode: ExperimentMode::Derived(DerivedExperiment {
                input_ids: &[
                    "t4b_hist_live",
                    "t4b_focus_direct",
                    "q3vl_2b_viterbi",
                    "q35_9b_viterbi",
                ],
            }),
        },
    ]
}

fn parse_experiment_filter() -> Vec<&'static str> {
    let Some(raw) = std::env::var("REALTEST_EXPERIMENTS").ok() else {
        return Vec::new();
    };
    let selected: Vec<String> = raw
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    build_suite()
        .into_iter()
        .map(|spec| spec.id)
        .filter(|id| selected.iter().any(|wanted| wanted == id))
        .collect()
}

fn discover_models(models_dir: &Path) -> HashMap<&'static str, ModelCandidate> {
    let mut models = HashMap::new();

    let known = [
        (
            "thinking4b",
            "Qwen3-VL-4B-Thinking",
            models_dir.join("Qwen3-VL-4B-Thinking/Qwen3VL-4B-Thinking-Q4_K_M.gguf"),
            models_dir.join("Qwen3-VL-4B-Thinking/mmproj-Qwen3VL-4B-Thinking-F16.gguf"),
            Some("none"),
        ),
        (
            "qwen35_4b",
            "Qwen3.5-4B",
            models_dir.join("unsloth_Qwen3.5-4B-GGUF/Qwen3.5-4B-Q4_K_M.gguf"),
            models_dir.join("unsloth_Qwen3.5-4B-GGUF/mmproj-F16.gguf"),
            Some("none"),
        ),
        (
            "qwen35_9b",
            "Qwen3.5-9B",
            models_dir.join("unsloth_Qwen3.5-9B-GGUF/Qwen3.5-9B-Q4_K_M.gguf"),
            models_dir.join("unsloth_Qwen3.5-9B-GGUF/mmproj-F16.gguf"),
            Some("none"),
        ),
        (
            "qwen3vl_2b",
            "Qwen3-VL-2B-Instruct",
            models_dir.join("Qwen_Qwen3-VL-2B-Instruct-GGUF/Qwen3VL-2B-Instruct-Q4_K_M.gguf"),
            models_dir.join("Qwen_Qwen3-VL-2B-Instruct-GGUF/mmproj-Qwen3VL-2B-Instruct-F16.gguf"),
            Some("none"),
        ),
    ];

    for (key, label, model_path, mmproj_path, reasoning_format) in known {
        if model_path.exists() && mmproj_path.exists() {
            models.insert(
                key,
                ModelCandidate {
                    key,
                    label,
                    model_path,
                    mmproj_path,
                    reasoning_format,
                },
            );
        }
    }

    maybe_insert_env_model(
        &mut models,
        "glm41v_9b",
        "GLM-4.1V-9B-Thinking",
        "REALTEST_MODEL_GLM41V_9B",
        "REALTEST_MMPROJ_GLM41V_9B",
        Some("none"),
    );
    maybe_insert_env_model(
        &mut models,
        "mangalmm",
        "MangaLMM",
        "REALTEST_MODEL_MANGALMM",
        "REALTEST_MMPROJ_MANGALMM",
        Some("none"),
    );

    models
}

fn maybe_insert_env_model(
    models: &mut HashMap<&'static str, ModelCandidate>,
    key: &'static str,
    label: &'static str,
    model_env: &str,
    mmproj_env: &str,
    reasoning_format: Option<&'static str>,
) {
    let Some(model_path) = std::env::var(model_env).ok().map(PathBuf::from) else {
        return;
    };
    let Some(mmproj_path) = std::env::var(mmproj_env).ok().map(PathBuf::from) else {
        return;
    };
    if model_path.exists() && mmproj_path.exists() {
        models.insert(
            key,
            ModelCandidate {
                key,
                label,
                model_path,
                mmproj_path,
                reasoning_format,
            },
        );
    }
}

async fn run_live_experiment(
    spec: &ExperimentSpec,
    live: &LiveExperiment,
    models: &HashMap<&'static str, ModelCandidate>,
    chapters: &BTreeMap<String, Vec<PageEntry>>,
    results_dir: &Path,
    verbose: bool,
) -> ExperimentSummary {
    let Some(model) = models.get(live.model_key) else {
        return ExperimentSummary {
            id: spec.id.to_string(),
            label: spec.label.to_string(),
            source: "live".to_string(),
            status: "skipped".to_string(),
            model: live.model_key.to_string(),
            prep: live.prep.as_str().to_string(),
            decision: live.decision.as_str().to_string(),
            sampling: live.sampling.name.to_string(),
            runtime: live.runtime.name.to_string(),
            strict: 0,
            relaxed: 0,
            intensity: 0,
            total: 0,
            errors: 0,
            second_pass_pages: 0,
            avg_window_s: 0.0,
            min_window_s: 0.0,
            max_window_s: 0.0,
            vram_mib: None,
            cache_path: None,
            notes: format!("Model '{}' not found locally", live.model_key),
            per_mood: BTreeMap::new(),
        };
    };

    println!(
        "  {CYAN}{BOLD}▶ {}{RESET} {DIM}[{} | {} | {} | {} | {} | {}]{RESET}",
        spec.label,
        model.label,
        live.prep.as_str(),
        live.context.as_str(),
        live.prompt_style.as_str(),
        live.decision.as_str(),
        live.runtime.name
    );

    let server_result = LlamaServer::start_with_options(
        model.model_path.to_str().unwrap(),
        model.mmproj_path.to_str().unwrap(),
        LlamaServerStartOptions {
            reasoning_format: model.reasoning_format.map(str::to_string),
            context_size: Some(live.runtime.context_size),
            parallel_slots: Some(live.runtime.parallel_slots),
            gpu_layers: None,
            runtime_intent: Some(live.runtime.intent),
        },
    )
    .await;

    let mut server = match server_result {
        Ok(server) => server,
        Err(err) => {
            return ExperimentSummary {
                id: spec.id.to_string(),
                label: spec.label.to_string(),
                source: "live".to_string(),
                status: "error".to_string(),
                model: model.label.to_string(),
                prep: live.prep.as_str().to_string(),
                decision: live.decision.as_str().to_string(),
                sampling: live.sampling.name.to_string(),
                runtime: live.runtime.name.to_string(),
                strict: 0,
                relaxed: 0,
                intensity: 0,
                total: 0,
                errors: 1,
                second_pass_pages: 0,
                avg_window_s: 0.0,
                min_window_s: 0.0,
                max_window_s: 0.0,
                vram_mib: None,
                cache_path: None,
                notes: format!("Server start failed: {err}"),
                per_mood: BTreeMap::new(),
            };
        }
    };

    let server_url = format!("http://127.0.0.1:{}/v1/chat/completions", server.port);
    let http = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(180))
        .build()
        .unwrap();
    let mut encoding_cache: HashMap<String, EncodedImage> = HashMap::new();
    let vram_mib = query_gpu_memory_used_mib();
    let mut window_durations = Vec::new();
    let mut total_errors = 0u32;
    let mut second_pass_pages = 0u32;
    let mut extra_time_total = 0.0f64;
    let mut result_map = HashMap::new();

    for (chapter_index, (chapter_key, pages)) in chapters.iter().enumerate() {
        println!(
            "    {DIM}[{}/{}] {} ({} pages){RESET}",
            chapter_index + 1,
            chapters.len(),
            chapter_key,
            pages.len()
        );

        let (window_results, window_errors) = run_window_pass(
            &http,
            &server_url,
            pages,
            live,
            &mut encoding_cache,
            verbose,
        )
        .await;
        total_errors += window_errors;
        window_durations.extend(window_results.iter().map(|w| w.elapsed_s));

        let vote_stats: Vec<PageVoteStats> = (0..pages.len())
            .map(|idx| build_vote_stats(&window_results, idx))
            .collect();

        let mut predictions = match live.decision {
            DecisionStrategy::Majority => aggregate_majority(pages, &vote_stats),
            DecisionStrategy::CenterOverride => aggregate_center_override(pages, &vote_stats),
            DecisionStrategy::Direct => aggregate_direct(&window_results, pages.len()),
            DecisionStrategy::Wide5SelectiveRepair => aggregate_majority(pages, &vote_stats),
            DecisionStrategy::Viterbi
            | DecisionStrategy::FocusedReprompt
            | DecisionStrategy::OcrReprompt
            | DecisionStrategy::SemanticReprompt => aggregate_viterbi(&vote_stats),
        };

        if matches!(live.decision, DecisionStrategy::Wide5SelectiveRepair) {
            let (repaired, extra_pages, extra_time) = apply_wide5_selective_repairs(
                &http,
                &server_url,
                pages,
                predictions,
                live,
                &mut encoding_cache,
            )
            .await;
            predictions = repaired;
            second_pass_pages += extra_pages;
            extra_time_total += extra_time;
        }

        if matches!(
            live.decision,
            DecisionStrategy::FocusedReprompt
                | DecisionStrategy::OcrReprompt
                | DecisionStrategy::SemanticReprompt
        ) {
            for idx in 0..pages.len() {
                if !is_ambiguous_prediction(&predictions[idx], &vote_stats[idx]) {
                    continue;
                }
                second_pass_pages += 1;
                let replacement = match live.decision {
                    DecisionStrategy::FocusedReprompt => {
                        reprompt_focus_page(
                            &http,
                            &server_url,
                            pages,
                            idx,
                            live,
                            &mut encoding_cache,
                        )
                        .await
                    }
                    DecisionStrategy::OcrReprompt => {
                        reprompt_with_ocr(
                            &http,
                            &server_url,
                            pages,
                            idx,
                            live,
                            &mut encoding_cache,
                        )
                        .await
                    }
                    DecisionStrategy::SemanticReprompt => {
                        reprompt_semantic_axes(
                            &http,
                            &server_url,
                            pages,
                            idx,
                            live,
                            &mut encoding_cache,
                        )
                        .await
                    }
                    _ => None,
                };

                if let Some(mut prediction) = replacement {
                    prediction.note = Some("second_pass".to_string());
                    predictions[idx] = prediction;
                }
            }
        }

        for (page, prediction) in pages.iter().zip(predictions.iter()) {
            result_map.insert(
                page.rel_path.clone(),
                format!("{}:{}", prediction.mood, prediction.intensity),
            );
        }
    }

    server.stop();
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let cache_path = results_dir.join(format!("realtest_suite_{}.json", spec.id));
    let _ = std::fs::write(
        &cache_path,
        serde_json::to_string_pretty(&result_map).unwrap(),
    );

    let effective_durations = if extra_time_total > 0.0 {
        let total_pages: usize = chapters.values().map(|pages| pages.len()).sum();
        let effective_avg =
            (window_durations.iter().sum::<f64>() + extra_time_total) / total_pages.max(1) as f64;
        vec![effective_avg; total_pages.max(1)]
    } else {
        window_durations.clone()
    };

    build_summary_from_results(
        spec,
        "live",
        model.label,
        live.prep.as_str(),
        live.decision.as_str(),
        live.sampling.name,
        live.runtime.name,
        total_errors,
        second_pass_pages,
        &effective_durations,
        vram_mib,
        &cache_path,
        spec.notes,
        chapters,
        &result_map,
    )
}

fn run_derived_experiment(
    spec: &ExperimentSpec,
    derived: &DerivedExperiment,
    chapters: &BTreeMap<String, Vec<PageEntry>>,
    results_dir: &Path,
    previous_summaries: &[ExperimentSummary],
) -> ExperimentSummary {
    let weights: HashMap<&'static str, f64> = HashMap::from([
        ("t4b_hist_live", 3.0),
        ("t4b_focus_direct", 2.0),
        ("q3vl_2b_viterbi", 2.0),
        ("q35_9b_viterbi", 1.0),
    ]);
    let lambda = 0.5f64;

    let mut inputs: HashMap<&'static str, HashMap<String, String>> = HashMap::new();
    for &input_id in derived.input_ids {
        let cache_path = results_dir.join(format!("realtest_suite_{}.json", input_id));
        let Some(result_map) = load_result_map(&cache_path) else {
            return ExperimentSummary {
                id: spec.id.to_string(),
                label: spec.label.to_string(),
                source: "derived".to_string(),
                status: "skipped".to_string(),
                model: "ensemble".to_string(),
                prep: "mixed".to_string(),
                decision: "weighted_viterbi".to_string(),
                sampling: "derived".to_string(),
                runtime: "sequential_ensemble".to_string(),
                strict: 0,
                relaxed: 0,
                intensity: 0,
                total: 0,
                errors: 0,
                second_pass_pages: 0,
                avg_window_s: 0.0,
                min_window_s: 0.0,
                max_window_s: 0.0,
                vram_mib: None,
                cache_path: None,
                notes: format!("Missing dependency result map: {}", cache_path.display()),
                per_mood: BTreeMap::new(),
            };
        };
        inputs.insert(input_id, result_map);
    }

    let mut result_map = HashMap::new();
    for pages in chapters.values() {
        let ordered_paths = pages
            .iter()
            .map(|page| page.rel_path.as_str())
            .collect::<Vec<_>>();
        let fused_moods = fuse_weighted_viterbi(&ordered_paths, &inputs, &weights, lambda);
        for (page, mood) in pages.iter().zip(fused_moods.into_iter()) {
            let intensity = inputs
                .get("q3vl_2b_viterbi")
                .and_then(|map| map.get(&page.rel_path))
                .and_then(|tag| parse_tag(tag).map(|(_, intensity)| intensity))
                .or_else(|| {
                    inputs
                        .get("t4b_hist_live")
                        .and_then(|map| map.get(&page.rel_path))
                        .and_then(|tag| parse_tag(tag).map(|(_, intensity)| intensity))
                })
                .unwrap_or(2);
            result_map.insert(page.rel_path.clone(), format!("{}:{}", mood, intensity));
        }
    }

    let cache_path = results_dir.join(format!("realtest_suite_{}.json", spec.id));
    let _ = std::fs::write(&cache_path, serde_json::to_string_pretty(&result_map).unwrap());

    let effective_avg = previous_summaries
        .iter()
        .filter(|summary| derived.input_ids.iter().any(|&id| id == summary.id))
        .map(|summary| summary.avg_window_s)
        .sum::<f64>()
        .max(0.0);
    let effective_vram = previous_summaries
        .iter()
        .filter(|summary| derived.input_ids.iter().any(|&id| id == summary.id))
        .filter_map(|summary| summary.vram_mib)
        .max();
    let total_pages: usize = chapters.values().map(|pages| pages.len()).sum();
    let durations = if total_pages == 0 {
        Vec::new()
    } else {
        vec![effective_avg; total_pages]
    };

    build_summary_from_results(
        spec,
        "derived",
        "ensemble(weighted_viterbi)",
        "mixed",
        "weighted_viterbi",
        "derived",
        "sequential_ensemble",
        0,
        0,
        &durations,
        effective_vram,
        &cache_path,
        &format!(
            "{} weights(hist=3, focus=2, q3=2, q35=1), lambda={:.2}",
            spec.notes, lambda
        ),
        chapters,
        &result_map,
    )
}

fn load_result_map(path: &Path) -> Option<HashMap<String, String>> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str::<HashMap<String, String>>(&text).ok())
}

fn parse_tag(tag: &str) -> Option<(String, u8)> {
    let mut parts = tag.split(':');
    let mood = parts.next()?.trim().to_string();
    let intensity = parts.next()?.trim().parse::<u8>().ok()?;
    Some((mood, intensity))
}

fn fuse_weighted_viterbi(
    ordered_paths: &[&str],
    inputs: &HashMap<&'static str, HashMap<String, String>>,
    weights: &HashMap<&'static str, f64>,
    lambda: f64,
) -> Vec<String> {
    if ordered_paths.is_empty() {
        return Vec::new();
    }

    let moods = BaseMood::ALL;
    let n = ordered_paths.len();
    let mut dp = vec![[f64::NEG_INFINITY; 8]; n];
    let mut back = vec![[0usize; 8]; n];

    let unary = |path: &str, mood: BaseMood| -> f64 {
        let mut score = 0.0;
        for (&input_id, &weight) in weights {
            let pred = inputs
                .get(input_id)
                .and_then(|map| map.get(path))
                .and_then(|tag| parse_tag(tag).map(|(mood, _)| mood))
                .unwrap_or_else(|| "???".to_string());
            if pred == mood.as_str() {
                score += weight;
            } else if pred == "???" {
                score -= 0.4 * weight;
            }
        }
        score.max(1e-6)
    };

    for &mood in moods.iter() {
        dp[0][mood.index()] = unary(ordered_paths[0], mood).ln();
    }

    for idx in 1..n {
        for &mood in moods.iter() {
            let unary_score = unary(ordered_paths[idx], mood).ln();
            let mut best_prev = 0usize;
            let mut best_score = f64::NEG_INFINITY;
            for &prev in moods.iter() {
                let transition = TRANSITION_MATRIX[prev.index()][mood.index()].max(0.05) as f64;
                let score = dp[idx - 1][prev.index()] + lambda * transition.ln() + unary_score;
                if score > best_score {
                    best_score = score;
                    best_prev = prev.index();
                }
            }
            dp[idx][mood.index()] = best_score;
            back[idx][mood.index()] = best_prev;
        }
    }

    let mut states = vec![0usize; n];
    let mut last_state = 0usize;
    let mut last_score = f64::NEG_INFINITY;
    for (state, score) in dp[n - 1].iter().enumerate() {
        if *score > last_score {
            last_score = *score;
            last_state = state;
        }
    }
    states[n - 1] = last_state;
    for idx in (1..n).rev() {
        states[idx - 1] = back[idx][states[idx]];
    }

    states
        .into_iter()
        .map(|state| BaseMood::ALL[state].as_str().to_string())
        .collect()
}

async fn run_window_pass(
    http: &reqwest::Client,
    server_url: &str,
    pages: &[PageEntry],
    live: &LiveExperiment,
    encoding_cache: &mut HashMap<String, EncodedImage>,
    verbose: bool,
) -> (Vec<WindowPrediction>, u32) {
    let mut results = Vec::new();
    let mut errors = 0u32;

    for center in 0..pages.len() {
        let left = center.saturating_sub(1);
        let right = if center + 1 < pages.len() {
            center + 1
        } else {
            center
        };
        let context_indices = live.context.indices(center, pages.len());
        let prompt = match live.prompt_style {
            PromptStyle::SequenceWindow => {
                build_historical_prompt(left + 1, center + 1, right + 1, pages.len())
            }
            PromptStyle::WideSequenceWindow => {
                build_wide_sequence_prompt(&context_indices, pages.len())
            }
            PromptStyle::CenterFocus => build_focus_prompt(center, pages.len()),
            PromptStyle::NarrativeFocus => build_narrative_focus_prompt(center, pages.len()),
            PromptStyle::WideNarrativeFocus => {
                build_wide_narrative_focus_prompt(&context_indices, center, pages.len())
            }
        };
        let images = context_indices
            .iter()
            .enumerate()
            .map(|(slot, &idx)| {
                let role = if slot == context_indices.len() / 2 {
                    ImageRole::Center
                } else {
                    ImageRole::Context
                };
                encode_page(&pages[idx], live.prep, role, encoding_cache)
            })
            .collect::<Vec<_>>();

        let body = build_window_body(live, &prompt, &images, live.grammar);

        let start = std::time::Instant::now();
        let json = post_json_with_retry(http, server_url, &body).await;
        let elapsed = start.elapsed().as_secs_f64();
        let (mood, intensity) = match json.as_ref().and_then(parse_mood_response_with_fallback) {
            Some(v) => v,
            None => {
                errors += 1;
                ("???".to_string(), 0)
            }
        };

        if verbose || center == 0 || center + 1 == pages.len() || (center + 1) % 10 == 0 {
            let context_label = context_indices
                .iter()
                .map(|idx| idx.to_string())
                .collect::<Vec<_>>()
                .join(",");
            println!(
                "      {DIM}[{}/{}] window [{}] → {:.1}s → {} {}{RESET}",
                center + 1,
                pages.len(),
                context_label,
                elapsed,
                mood,
                intensity
            );
        }

        results.push(WindowPrediction {
            members: context_indices,
            left,
            center,
            right,
            mood,
            intensity,
            elapsed_s: elapsed,
        });
    }

    (results, errors)
}

fn build_window_body(
    live: &LiveExperiment,
    prompt: &str,
    images: &[EncodedImage],
    grammar_enabled: bool,
) -> serde_json::Value {
    let mut content = images
        .iter()
        .map(|image| {
            serde_json::json!({
                "type": "image_url",
                "image_url": { "url": format!("data:{};base64,{}", image.mime, image.b64) }
            })
        })
        .collect::<Vec<_>>();
    content.push(serde_json::json!({ "type": "text", "text": prompt }));

    let mut body = serde_json::json!({
        "model": "test",
        "messages": [{
            "role": "user",
            "content": content
        }],
        "max_tokens": live.sampling.max_tokens,
        "temperature": live.sampling.temperature
    });

    if let Some(top_p) = live.sampling.top_p {
        body["top_p"] = serde_json::json!(top_p);
    }
    if let Some(top_k) = live.sampling.top_k {
        body["top_k"] = serde_json::json!(top_k);
    }
    if let Some(seed) = live.sampling.seed {
        body["seed"] = serde_json::json!(seed);
    }
    if grammar_enabled {
        body["grammar"] = serde_json::json!(MOOD_GRAMMAR);
    }

    body
}

fn build_historical_prompt(
    page_start: usize,
    page_center: usize,
    page_end: usize,
    total_pages: usize,
) -> String {
    format!(
        "These are 3 consecutive manga pages from the same chapter (pages {}, {}, {} out of {}).\n\
        They are shown in reading order: page LEFT, page CENTER, page RIGHT.\n\
        \n\
        Considering the flow across all 3 pages, what is the overall mood of this sequence for soundtrack purposes?\n\
        \n\
        Classify as ONE mood that best represents the group:\n\
        - epic: climactic moments, declarations of resolve, characters unleashing power, pivotal turning points, peak dramatic intensity\n\
        - tension: buildup, uncertainty, standoffs, threats without resolution, ominous atmosphere, characters evaluating the situation\n\
        - sadness: loss, grief, crying, emotional pain, defeat\n\
        - comedy: comic relief, gag reactions, slapstick, funny situations (NOT just smiling or friendly characters)\n\
        - romance: love, intimacy, tender moments between characters\n\
        - horror: fear, gore, monsters, nightmarish imagery\n\
        - peaceful: calm scenes, daily life, quiet contemplation, friendly conversations\n\
        - mystery: secrets revealed, scheming, hidden motives, ominous foreshadowing\n\
        \n\
        Rate the intensity from 1 (low) to 3 (high).\n\
        \n\
        Reply format: mood intensity\n\
        Example: tension 2",
        page_start, page_center, page_end, total_pages
    )
}

fn build_focus_prompt(page_idx: usize, total_pages: usize) -> String {
    format!(
        "These are 3 consecutive manga pages from the same chapter.\n\
        Image 1 is the PREVIOUS page, Image 2 is the CURRENT page, Image 3 is the NEXT page.\n\
        \n\
        Classify the CURRENT page only (Image 2) for soundtrack purposes.\n\
        Use the neighboring pages only to disambiguate the current page.\n\
        \n\
        Current page number: {} out of {}.\n\
        \n\
        Reply format: mood intensity\n\
        Allowed moods: epic, tension, sadness, comedy, romance, horror, peaceful, mystery",
        page_idx + 1,
        total_pages
    )
}

fn build_wide_sequence_prompt(context_indices: &[usize], total_pages: usize) -> String {
    let page_numbers = context_indices
        .iter()
        .map(|idx| (idx + 1).to_string())
        .collect::<Vec<_>>()
        .join(", ");

    format!(
        "These are 5 consecutive manga pages from the same chapter (pages {} out of {}).\n\
        They are shown in reading order from earliest to latest.\n\
        \n\
        Considering the short narrative flow across all 5 pages, what is the dominant mood of this sequence for soundtrack purposes?\n\
        \n\
        Classify as ONE mood that best represents the 5-page segment:\n\
        - epic: climactic moments, declarations of resolve, power unleashed, clear payoff or triumph\n\
        - tension: buildup, uncertainty, standoffs, unresolved threats, pressure without release\n\
        - sadness: grief, defeat, crying, emotional pain, aftermath\n\
        - comedy: comic relief, gags, slapstick, funny reactions\n\
        - romance: intimacy, affection, tender moments\n\
        - horror: fear, monsters, gore, nightmare imagery\n\
        - peaceful: calm pause, daily life, quiet contemplation, friendly conversation\n\
        - mystery: secrets, scheming, ominous reveal, hidden motives, foreshadowing\n\
        \n\
        Rate the intensity from 1 (low) to 3 (high).\n\
        \n\
        Reply format: mood intensity\n\
        Example: tension 2",
        page_numbers,
        total_pages
    )
}

fn build_narrative_focus_prompt(page_idx: usize, total_pages: usize) -> String {
    format!(
        "These are 3 consecutive manga pages from the same chapter.\n\
        Image 1 is the PREVIOUS page, Image 2 is the CURRENT page, Image 3 is the NEXT page.\n\
        \n\
        Classify the CURRENT page only (Image 2) for soundtrack purposes.\n\
        Use neighboring pages only to understand the narrative function of the CURRENT page.\n\
        \n\
        First decide whether the CURRENT page is mainly:\n\
        - payoff / climax / release -> epic\n\
        - buildup / standoff / unresolved threat / evaluation -> tension\n\
        - grief / loss / crying / aftermath -> sadness\n\
        - comic relief / gag / silly reaction -> comedy\n\
        - tenderness / intimacy / affection -> romance\n\
        - fear / monster / nightmare / shock -> horror\n\
        - quiet pause / daily life / calm reflection -> peaceful\n\
        - reveal / secret / foreshadowing / scheming -> mystery\n\
        \n\
        Critical rules:\n\
        - action without release or payoff is tension, not epic\n\
        - hidden motives, reveals, and ominous setup are mystery, not epic\n\
        - a single quiet page can still be peaceful even between intense pages\n\
        - crying, regret, or aftermath are sadness, not epic\n\
        \n\
        Current page number: {} out of {}.\n\
        \n\
        Reply with ONLY: mood intensity\n\
        Example: tension 2",
        page_idx + 1,
        total_pages
    )
}

fn build_wide_narrative_focus_prompt(
    context_indices: &[usize],
    page_idx: usize,
    total_pages: usize,
) -> String {
    let numbered = context_indices
        .iter()
        .enumerate()
        .map(|(slot, idx)| format!("Image {} = page {}", slot + 1, idx + 1))
        .collect::<Vec<_>>()
        .join(", ");

    format!(
        "These are 5 nearby manga pages from the same chapter.\n\
        {numbered} out of {total_pages} total pages.\n\
        Image 3 is the CURRENT page to classify.\n\
        Images 1-2 are earlier context. Images 4-5 are future context.\n\
        \n\
        Classify the CURRENT page only for soundtrack purposes.\n\
        Use the surrounding pages only to understand whether the CURRENT page is setup, payoff, aftermath, reveal, or a quiet reset.\n\
        \n\
        Critical rules:\n\
        - Do NOT average the 5 pages into one mood.\n\
        - If the CURRENT page starts a mood shift, label the CURRENT page's new mood.\n\
        - Epic requires payoff, release, triumph, or a real turning-point on the CURRENT page.\n\
        - Action, pressure, threat, or confrontation without payoff on the CURRENT page is tension.\n\
        - Hidden motives, revelations, ominous hints, or scheming are mystery.\n\
        - A single calm reset page can still be peaceful even between intense pages.\n\
        - Grief, crying, regret, or aftermath are sadness, not epic.\n\
        \n\
        Current page number: {} out of {}.\n\
        \n\
        Reply with ONLY: mood intensity\n\
        Example: tension 2",
        page_idx + 1,
        total_pages
    )
}

fn build_ocr_prompt() -> &'static str {
    "Read the visible dialogue, narration, sound effects, or labels on this manga page. Reply with short plain-text lines only. If nothing is readable, reply UNREADABLE."
}

fn build_focus_with_ocr_prompt(page_idx: usize, total_pages: usize, ocr_text: &str) -> String {
    format!(
        "{}\n\nVisible text extracted from the current page:\n{}\n\nReply format: mood intensity",
        build_focus_prompt(page_idx, total_pages),
        ocr_text
    )
}

fn build_semantic_prompt() -> &'static str {
    "Rate the current manga page on these soundtrack axes from 0 to 3.\n\
    Reply EXACTLY with:\n\
    AWE=n URGENCY=n GRIEF=n LEVITY=n INTIMACY=n DREAD=n CALM=n ENIGMA=n"
}

fn encode_page(
    page: &PageEntry,
    prep: ImagePrepMode,
    role: ImageRole,
    cache: &mut HashMap<String, EncodedImage>,
) -> EncodedImage {
    let cache_key = format!("{}::{}::{:?}", page.rel_path, prep.as_str(), role);
    if let Some(existing) = cache.get(&cache_key) {
        return existing.clone();
    }

    let encoded = match prep {
        ImagePrepMode::Legacy672Jpeg => EncodedImage {
            mime: "image/jpeg",
            b64: inference::prepare_image(&page.raw_bytes)
                .unwrap_or_else(|e| panic!("Failed to prepare {}: {}", page.rel_path, e)),
        },
        ImagePrepMode::CenterPng1024 => {
            if matches!(role, ImageRole::Center) {
                encode_custom_image(&page.raw_bytes, 1024, image::ImageFormat::Png, "image/png")
            } else {
                EncodedImage {
                    mime: "image/jpeg",
                    b64: inference::prepare_image(&page.raw_bytes)
                        .unwrap_or_else(|e| panic!("Failed to prepare {}: {}", page.rel_path, e)),
                }
            }
        }
        ImagePrepMode::AllPng1024 => {
            encode_custom_image(&page.raw_bytes, 1024, image::ImageFormat::Png, "image/png")
        }
    };

    cache.insert(cache_key, encoded.clone());
    encoded
}

fn encode_custom_image(
    raw_bytes: &[u8],
    max_dim: u32,
    format: image::ImageFormat,
    mime: &'static str,
) -> EncodedImage {
    let img = image::load_from_memory(raw_bytes).expect("Failed to decode source image");
    let (w, h) = img.dimensions();
    let resized = if w > max_dim || h > max_dim {
        let scale = max_dim as f64 / w.max(h) as f64;
        let new_w = (w as f64 * scale).round() as u32;
        let new_h = (h as f64 * scale).round() as u32;
        img.resize(new_w, new_h, image::imageops::FilterType::Lanczos3)
    } else {
        img
    };

    let mut cursor = std::io::Cursor::new(Vec::new());
    resized.write_to(&mut cursor, format).unwrap();
    EncodedImage {
        mime,
        b64: base64::engine::general_purpose::STANDARD.encode(cursor.into_inner()),
    }
}

async fn post_json_with_retry(
    http: &reqwest::Client,
    server_url: &str,
    body: &serde_json::Value,
) -> Option<serde_json::Value> {
    for attempt in 1..=3u32 {
        if let Ok(resp) = http.post(server_url).json(body).send().await {
            if resp.status().is_success() {
                if let Ok(json) = resp.json::<serde_json::Value>().await {
                    return Some(json);
                }
            }
        }
        if attempt < 3 {
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }
    }
    None
}

fn parse_mood_response_with_fallback(json: &serde_json::Value) -> Option<(String, u8)> {
    match parse_mood_intensity_response(json) {
        Ok(tag) => Some((tag.mood.as_str().to_string(), tag.intensity.as_u8())),
        Err(_) => {
            let raw_content = extract_content(json).unwrap_or_default().to_lowercase();
            let cleaned = if let Some(pos) = raw_content.find("</think>") {
                &raw_content[pos + 8..]
            } else {
                raw_content.as_str()
            };
            let mut best: Option<(BaseMood, usize)> = None;
            for &mood in BaseMood::ALL.iter() {
                if let Some(pos) = cleaned.rfind(mood.as_str()) {
                    if best.is_none() || pos > best.unwrap().1 {
                        best = Some((mood, pos));
                    }
                }
            }
            best.map(|(mood, _)| (mood.as_str().to_string(), 2))
        }
    }
}

fn build_vote_stats(window_results: &[WindowPrediction], page_idx: usize) -> PageVoteStats {
    let mut counts = [0u32; 8];
    let mut weighted = [0.0f64; 8];
    let mut intensity_sum = [0u32; 8];
    let mut intensity_count = [0u32; 8];
    let mut center_vote = None;
    let mut center_intensity = None;

    for vote in window_results
        .iter()
        .filter(|vote| vote.members.iter().any(|member| *member == page_idx))
    {
        let Some(mood) = BaseMood::from_str_opt(&vote.mood) else {
            continue;
        };
        let idx = mood.index();
        counts[idx] += 1;
        let role_weight = if vote.center == page_idx { 1.35 } else { 1.0 };
        weighted[idx] += role_weight;
        intensity_sum[idx] += vote.intensity as u32;
        intensity_count[idx] += 1;
        if vote.center == page_idx {
            center_vote = Some(mood);
            center_intensity = Some(vote.intensity);
        }
    }

    let mut sorted_scores = weighted
        .iter()
        .enumerate()
        .map(|(idx, score)| (idx, *score))
        .collect::<Vec<_>>();
    sorted_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    let winner_score = sorted_scores.first().map(|(_, score)| *score).unwrap_or(0.0);
    let runner_up_score = sorted_scores.get(1).map(|(_, score)| *score).unwrap_or(0.0);

    PageVoteStats {
        counts,
        weighted,
        intensity_sum,
        intensity_count,
        center_vote,
        center_intensity,
        winner_score,
        runner_up_score,
    }
}

fn aggregate_majority(pages: &[PageEntry], stats: &[PageVoteStats]) -> Vec<PagePrediction> {
    pages.iter()
        .enumerate()
        .map(|(idx, _)| {
            let page_stats = &stats[idx];
            let mut sorted = page_stats
                .counts
                .iter()
                .enumerate()
                .map(|(mood_idx, count)| (mood_idx, *count))
                .collect::<Vec<_>>();
            sorted.sort_by(|a, b| b.1.cmp(&a.1));

            let winner_idx = if sorted.first().map(|(_, c)| *c).unwrap_or(0) == 0 {
                BaseMood::Tension.index()
            } else if sorted.len() > 1 && sorted[0].1 == sorted[1].1 {
                page_stats
                    .center_vote
                    .map(|mood| mood.index())
                    .unwrap_or(sorted[0].0)
            } else {
                sorted[0].0
            };

            let mood = BaseMood::ALL[winner_idx].as_str().to_string();
            let intensity = average_intensity(page_stats, winner_idx).unwrap_or(2);
            PagePrediction {
                mood,
                intensity,
                winner_score: page_stats.winner_score,
                runner_up_score: page_stats.runner_up_score,
                note: None,
            }
        })
        .collect()
}

fn aggregate_center_override(pages: &[PageEntry], stats: &[PageVoteStats]) -> Vec<PagePrediction> {
    pages.iter()
        .enumerate()
        .map(|(idx, _)| {
            let page_stats = &stats[idx];
            let mut sorted = page_stats
                .counts
                .iter()
                .enumerate()
                .map(|(mood_idx, count)| (mood_idx, *count))
                .collect::<Vec<_>>();
            sorted.sort_by(|a, b| b.1.cmp(&a.1));

            let majority_idx = if sorted.first().map(|(_, c)| *c).unwrap_or(0) == 0 {
                BaseMood::Tension.index()
            } else if sorted.len() > 1 && sorted[0].1 == sorted[1].1 {
                page_stats
                    .center_vote
                    .map(|mood| mood.index())
                    .unwrap_or(sorted[0].0)
            } else {
                sorted[0].0
            };

            let majority_mood = BaseMood::ALL[majority_idx];
            let mut final_idx = majority_idx;
            let mut note = None;

            if let Some(center_mood) = page_stats.center_vote {
                let center_idx = center_mood.index();
                if center_idx != majority_idx {
                    let majority_count = page_stats.counts[majority_idx];
                    let center_count = page_stats.counts[center_idx];
                    let center_intensity = page_stats.center_intensity.unwrap_or(2);
                    let fragile_majority = majority_count <= center_count + 1;
                    let center_is_specific = matches!(
                        center_mood,
                        BaseMood::Mystery
                            | BaseMood::Peaceful
                            | BaseMood::Comedy
                            | BaseMood::Sadness
                            | BaseMood::Romance
                            | BaseMood::Horror
                    );
                    let action_correction = matches!(
                        (majority_mood, center_mood),
                        (BaseMood::Epic, BaseMood::Tension)
                            | (BaseMood::Comedy, BaseMood::Tension)
                            | (BaseMood::Comedy, BaseMood::Mystery)
                            | (BaseMood::Comedy, BaseMood::Sadness)
                            | (BaseMood::Horror, BaseMood::Tension)
                    ) && center_intensity <= 2;

                    if fragile_majority && (center_is_specific || action_correction) {
                        final_idx = center_idx;
                        note = Some(format!(
                            "center_override:{}>{}",
                            center_mood.as_str(),
                            majority_mood.as_str()
                        ));
                    }
                }
            }

            let intensity = if final_idx == page_stats.center_vote.map(|m| m.index()).unwrap_or(99) {
                page_stats
                    .center_intensity
                    .unwrap_or_else(|| average_intensity(page_stats, final_idx).unwrap_or(2))
            } else {
                average_intensity(page_stats, final_idx).unwrap_or(2)
            };

            PagePrediction {
                mood: BaseMood::ALL[final_idx].as_str().to_string(),
                intensity,
                winner_score: page_stats.winner_score,
                runner_up_score: page_stats.runner_up_score,
                note,
            }
        })
        .collect()
}

fn aggregate_direct(window_results: &[WindowPrediction], page_count: usize) -> Vec<PagePrediction> {
    let mut predictions = vec![
        PagePrediction {
            mood: "???".to_string(),
            intensity: 0,
            winner_score: 0.0,
            runner_up_score: 0.0,
            note: Some("missing_center".to_string()),
        };
        page_count
    ];

    for window in window_results {
        predictions[window.center] = PagePrediction {
            mood: window.mood.clone(),
            intensity: window.intensity,
            winner_score: 1.0,
            runner_up_score: 0.0,
            note: Some("direct_center".to_string()),
        };
    }

    predictions
}

fn aggregate_viterbi(stats: &[PageVoteStats]) -> Vec<PagePrediction> {
    if stats.is_empty() {
        return Vec::new();
    }

    let lambda = 0.35f64;
    let moods = BaseMood::ALL;
    let n = stats.len();
    let mut dp = vec![[f64::NEG_INFINITY; 8]; n];
    let mut back = vec![[0usize; 8]; n];

    for &mood in moods.iter() {
        dp[0][mood.index()] = unary_score(&stats[0], mood).ln();
    }

    for idx in 1..n {
        for &mood in moods.iter() {
            let unary = unary_score(&stats[idx], mood).ln();
            let mut best_prev = 0usize;
            let mut best_score = f64::NEG_INFINITY;
            for &prev in moods.iter() {
                let transition = TRANSITION_MATRIX[prev.index()][mood.index()].max(0.05) as f64;
                let score = dp[idx - 1][prev.index()] + lambda * transition.ln() + unary;
                if score > best_score {
                    best_score = score;
                    best_prev = prev.index();
                }
            }
            dp[idx][mood.index()] = best_score;
            back[idx][mood.index()] = best_prev;
        }
    }

    let mut states = vec![0usize; n];
    let (last_state, _) = dp[n - 1]
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
        .unwrap();
    states[n - 1] = last_state;
    for idx in (1..n).rev() {
        states[idx - 1] = back[idx][states[idx]];
    }

    states
        .iter()
        .enumerate()
        .map(|(idx, state)| {
            let mood = BaseMood::ALL[*state].as_str().to_string();
            let intensity = average_intensity(&stats[idx], *state).unwrap_or(2);
            PagePrediction {
                mood,
                intensity,
                winner_score: stats[idx].winner_score,
                runner_up_score: stats[idx].runner_up_score,
                note: None,
            }
        })
        .collect()
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
enum SelectivePrompt {
    Focus,
    Narrative,
}

impl SelectivePrompt {
    fn note(self) -> &'static str {
        match self {
            Self::Focus => "selective_focus",
            Self::Narrative => "selective_narrative",
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct MoodRun {
    start: usize,
    end: usize,
}

impl MoodRun {
    fn len(self) -> usize {
        self.end - self.start + 1
    }
}

async fn apply_wide5_selective_repairs(
    http: &reqwest::Client,
    server_url: &str,
    pages: &[PageEntry],
    mut predictions: Vec<PagePrediction>,
    live: &LiveExperiment,
    cache: &mut HashMap<String, EncodedImage>,
) -> (Vec<PagePrediction>, u32, f64) {
    let base_moods = predictions
        .iter()
        .map(|prediction| prediction.mood.clone())
        .collect::<Vec<_>>();
    let runs = build_mood_runs(&base_moods);
    let mut reprompt_cache: HashMap<(usize, SelectivePrompt), Option<(PagePrediction, f64)>> =
        HashMap::new();
    let mut touched_pages = HashSet::new();
    let mut extra_time = 0.0f64;

    for (run_idx, run) in runs.iter().copied().enumerate() {
        let mood = base_moods[run.start].as_str();
        let next_run = runs.get(run_idx + 1).copied();

        if mood == "comedy" && run.len() >= 4 {
            for idx in [run.start, run.end] {
                if let Some(prediction) = get_selective_prediction(
                    http,
                    server_url,
                    pages,
                    idx,
                    SelectivePrompt::Focus,
                    live,
                    cache,
                    &mut reprompt_cache,
                    &mut touched_pages,
                    &mut extra_time,
                )
                .await
                {
                    if matches!(prediction.mood.as_str(), "tension" | "mystery") {
                        apply_selective_override(&mut predictions[idx], prediction, "comedy_edge");
                    }
                }
            }
        }

        if mood == "tension" && run.len() >= 10 {
            let first_a = run.start;
            let first_b = run.start + 1;
            let last_a = run.end - 1;
            let last_b = run.end;

            let first_left = get_selective_prediction(
                http,
                server_url,
                pages,
                first_a,
                SelectivePrompt::Narrative,
                live,
                cache,
                &mut reprompt_cache,
                &mut touched_pages,
                &mut extra_time,
            )
            .await;
            let first_right = get_selective_prediction(
                http,
                server_url,
                pages,
                first_b,
                SelectivePrompt::Narrative,
                live,
                cache,
                &mut reprompt_cache,
                &mut touched_pages,
                &mut extra_time,
            )
            .await;
            if first_left.as_ref().map(|p| p.mood.as_str()) == Some("mystery")
                && first_right.as_ref().map(|p| p.mood.as_str()) == Some("mystery")
            {
                if let Some(prediction) = first_left {
                    apply_selective_override(
                        &mut predictions[first_a],
                        prediction,
                        "tension_run_open_mystery",
                    );
                }
                if let Some(prediction) = first_right {
                    apply_selective_override(
                        &mut predictions[first_b],
                        prediction,
                        "tension_run_open_mystery",
                    );
                }
            }

            let last_left = get_selective_prediction(
                http,
                server_url,
                pages,
                last_a,
                SelectivePrompt::Narrative,
                live,
                cache,
                &mut reprompt_cache,
                &mut touched_pages,
                &mut extra_time,
            )
            .await;
            let last_right = get_selective_prediction(
                http,
                server_url,
                pages,
                last_b,
                SelectivePrompt::Narrative,
                live,
                cache,
                &mut reprompt_cache,
                &mut touched_pages,
                &mut extra_time,
            )
            .await;
            if last_left.as_ref().map(|p| p.mood.as_str()) == Some("epic")
                && last_right.as_ref().map(|p| p.mood.as_str()) == Some("epic")
            {
                if let Some(prediction) = last_left {
                    apply_selective_override(
                        &mut predictions[last_a],
                        prediction,
                        "tension_run_close_epic",
                    );
                }
                if let Some(prediction) = last_right {
                    apply_selective_override(
                        &mut predictions[last_b],
                        prediction,
                        "tension_run_close_epic",
                    );
                }
            }
        }

        if mood == "epic" && run.len() <= 4 && run.start > 0 {
            if let Some(prediction) = get_selective_prediction(
                http,
                server_url,
                pages,
                run.start,
                SelectivePrompt::Narrative,
                live,
                cache,
                &mut reprompt_cache,
                &mut touched_pages,
                &mut extra_time,
            )
            .await
            {
                if prediction.mood == "tension" {
                    apply_selective_override(
                        &mut predictions[run.start],
                        prediction,
                        "short_epic_open_tension",
                    );
                }
            }
        }

        if mood == "epic"
            && run.len() >= 4
            && next_run
                .map(|next| base_moods[next.start].as_str() == "tension" && next.len() >= 10)
                .unwrap_or(false)
        {
            let penultimate = run.end.saturating_sub(1);
            if let Some(prediction) = get_selective_prediction(
                http,
                server_url,
                pages,
                penultimate,
                SelectivePrompt::Focus,
                live,
                cache,
                &mut reprompt_cache,
                &mut touched_pages,
                &mut extra_time,
            )
            .await
            {
                if prediction.mood == "mystery" {
                    apply_selective_override(
                        &mut predictions[penultimate],
                        prediction,
                        "epic_bridge_penultimate_mystery",
                    );
                }
            }

            if let Some(prediction) = get_selective_prediction(
                http,
                server_url,
                pages,
                run.end,
                SelectivePrompt::Narrative,
                live,
                cache,
                &mut reprompt_cache,
                &mut touched_pages,
                &mut extra_time,
            )
            .await
            {
                if prediction.mood == "mystery" {
                    apply_selective_override(
                        &mut predictions[run.end],
                        prediction,
                        "epic_bridge_final_mystery",
                    );
                }
            }
        }

        if mood == "epic" && run.len() >= 8 {
            let left_idx = run.end.saturating_sub(1);
            let right_idx = run.end;
            let left = get_selective_prediction(
                http,
                server_url,
                pages,
                left_idx,
                SelectivePrompt::Focus,
                live,
                cache,
                &mut reprompt_cache,
                &mut touched_pages,
                &mut extra_time,
            )
            .await;
            let right = get_selective_prediction(
                http,
                server_url,
                pages,
                right_idx,
                SelectivePrompt::Focus,
                live,
                cache,
                &mut reprompt_cache,
                &mut touched_pages,
                &mut extra_time,
            )
            .await;
            if left.as_ref().map(|p| p.mood.as_str()) == Some("tension")
                && right.as_ref().map(|p| p.mood.as_str()) == Some("tension")
            {
                if let Some(prediction) = left {
                    apply_selective_override(
                        &mut predictions[left_idx],
                        prediction,
                        "long_epic_tail_tension",
                    );
                }
                if let Some(prediction) = right {
                    apply_selective_override(
                        &mut predictions[right_idx],
                        prediction,
                        "long_epic_tail_tension",
                    );
                }
            }
        }
    }

    (predictions, touched_pages.len() as u32, extra_time)
}

fn build_mood_runs(moods: &[String]) -> Vec<MoodRun> {
    if moods.is_empty() {
        return Vec::new();
    }

    let mut runs = Vec::new();
    let mut start = 0usize;
    while start < moods.len() {
        let mut end = start;
        while end + 1 < moods.len() && moods[end + 1] == moods[start] {
            end += 1;
        }
        runs.push(MoodRun { start, end });
        start = end + 1;
    }
    runs
}

fn apply_selective_override(
    target: &mut PagePrediction,
    replacement: PagePrediction,
    reason: &str,
) {
    *target = PagePrediction {
        note: Some(reason.to_string()),
        ..replacement
    };
}

async fn get_selective_prediction(
    http: &reqwest::Client,
    server_url: &str,
    pages: &[PageEntry],
    idx: usize,
    prompt_kind: SelectivePrompt,
    live: &LiveExperiment,
    cache: &mut HashMap<String, EncodedImage>,
    reprompt_cache: &mut HashMap<(usize, SelectivePrompt), Option<(PagePrediction, f64)>>,
    touched_pages: &mut HashSet<usize>,
    extra_time: &mut f64,
) -> Option<PagePrediction> {
    let key = (idx, prompt_kind);
    if let Some(cached) = reprompt_cache.get(&key) {
        return cached.as_ref().map(|(prediction, _)| prediction.clone());
    }

    let result = selective_reprompt_page(http, server_url, pages, idx, prompt_kind, live, cache).await;
    reprompt_cache.insert(key, result.clone());

    if let Some((prediction, elapsed_s)) = result {
        touched_pages.insert(idx);
        *extra_time += elapsed_s;
        Some(prediction)
    } else {
        None
    }
}

async fn selective_reprompt_page(
    http: &reqwest::Client,
    server_url: &str,
    pages: &[PageEntry],
    idx: usize,
    prompt_kind: SelectivePrompt,
    live: &LiveExperiment,
    cache: &mut HashMap<String, EncodedImage>,
) -> Option<(PagePrediction, f64)> {
    let left = idx.saturating_sub(1);
    let right = if idx + 1 < pages.len() { idx + 1 } else { idx };
    let images = vec![
        encode_page(
            &pages[left],
            ImagePrepMode::CenterPng1024,
            ImageRole::Context,
            cache,
        ),
        encode_page(
            &pages[idx],
            ImagePrepMode::CenterPng1024,
            ImageRole::Center,
            cache,
        ),
        encode_page(
            &pages[right],
            ImagePrepMode::CenterPng1024,
            ImageRole::Context,
            cache,
        ),
    ];
    let prompt = match prompt_kind {
        SelectivePrompt::Focus => build_focus_prompt(idx, pages.len()),
        SelectivePrompt::Narrative => build_narrative_focus_prompt(idx, pages.len()),
    };
    let request_live = LiveExperiment {
        model_key: live.model_key,
        prep: ImagePrepMode::CenterPng1024,
        context: ContextWindow::Triad,
        prompt_style: match prompt_kind {
            SelectivePrompt::Focus => PromptStyle::CenterFocus,
            SelectivePrompt::Narrative => PromptStyle::NarrativeFocus,
        },
        sampling: SamplingConfig {
            name: match prompt_kind {
                SelectivePrompt::Focus => "selective_focus",
                SelectivePrompt::Narrative => "selective_narrative",
            },
            temperature: 0.0,
            top_p: None,
            top_k: None,
            seed: None,
            max_tokens: 512,
        },
        runtime: live.runtime,
        grammar: false,
        decision: DecisionStrategy::Direct,
    };
    let body = build_window_body(&request_live, &prompt, &images, false);
    let start = std::time::Instant::now();
    let json = post_json_with_retry(http, server_url, &body).await?;
    let elapsed = start.elapsed().as_secs_f64();
    let (mood, intensity) = parse_mood_response_with_fallback(&json)?;

    Some((
        PagePrediction {
            mood,
            intensity,
            winner_score: 0.0,
            runner_up_score: 0.0,
            note: Some(prompt_kind.note().to_string()),
        },
        elapsed,
    ))
}

fn unary_score(stats: &PageVoteStats, mood: BaseMood) -> f64 {
    stats.weighted[mood.index()].max(0.05)
}

fn average_intensity(stats: &PageVoteStats, mood_idx: usize) -> Option<u8> {
    let count = stats.intensity_count[mood_idx];
    if count == 0 {
        return None;
    }
    Some(
        ((stats.intensity_sum[mood_idx] as f64 / count as f64).round() as u8)
            .clamp(1, 3),
    )
}

fn is_ambiguous_prediction(prediction: &PagePrediction, stats: &PageVoteStats) -> bool {
    prediction.winner_score <= stats.runner_up_score + 0.75
        || prediction.winner_score < 2.0
        || matches!(
            prediction.mood.as_str(),
            "mystery" | "peaceful" | "romance" | "horror"
        )
}

async fn reprompt_focus_page(
    http: &reqwest::Client,
    server_url: &str,
    pages: &[PageEntry],
    idx: usize,
    live: &LiveExperiment,
    cache: &mut HashMap<String, EncodedImage>,
) -> Option<PagePrediction> {
    let left = idx.saturating_sub(1);
    let right = if idx + 1 < pages.len() { idx + 1 } else { idx };
    let left_img = encode_page(&pages[left], live.prep, ImageRole::Context, cache);
    let center_img = encode_page(&pages[idx], live.prep, ImageRole::Center, cache);
    let right_img = encode_page(&pages[right], live.prep, ImageRole::Context, cache);
    let prompt = build_focus_prompt(idx, pages.len());
    let images = vec![left_img, center_img, right_img];
    let body = build_window_body(live, &prompt, &images, true);
    let json = post_json_with_retry(http, server_url, &body).await?;
    let (mood, intensity) = parse_mood_response_with_fallback(&json)?;
    Some(PagePrediction {
        mood,
        intensity,
        winner_score: 0.0,
        runner_up_score: 0.0,
        note: Some("focus".to_string()),
    })
}

async fn reprompt_with_ocr(
    http: &reqwest::Client,
    server_url: &str,
    pages: &[PageEntry],
    idx: usize,
    live: &LiveExperiment,
    cache: &mut HashMap<String, EncodedImage>,
) -> Option<PagePrediction> {
    let center_img = encode_page(&pages[idx], live.prep, ImageRole::Center, cache);
    let ocr_body = serde_json::json!({
        "model": "test",
        "messages": [{
            "role": "user",
            "content": [
                { "type": "image_url", "image_url": { "url": format!("data:{};base64,{}", center_img.mime, center_img.b64) } },
                { "type": "text", "text": build_ocr_prompt() }
            ]
        }],
        "max_tokens": 192,
        "temperature": 0.0
    });
    let ocr_json = post_json_with_retry(http, server_url, &ocr_body).await?;
    let ocr_text = extract_content(&ocr_json)
        .unwrap_or_else(|| "UNREADABLE".to_string())
        .chars()
        .take(400)
        .collect::<String>();

    let left = idx.saturating_sub(1);
    let right = if idx + 1 < pages.len() { idx + 1 } else { idx };
    let left_img = encode_page(&pages[left], live.prep, ImageRole::Context, cache);
    let right_img = encode_page(&pages[right], live.prep, ImageRole::Context, cache);
    let prompt = build_focus_with_ocr_prompt(idx, pages.len(), &ocr_text);
    let images = vec![left_img, center_img, right_img];
    let body = build_window_body(live, &prompt, &images, true);
    let json = post_json_with_retry(http, server_url, &body).await?;
    let (mood, intensity) = parse_mood_response_with_fallback(&json)?;
    Some(PagePrediction {
        mood,
        intensity,
        winner_score: 0.0,
        runner_up_score: 0.0,
        note: Some("ocr".to_string()),
    })
}

async fn reprompt_semantic_axes(
    http: &reqwest::Client,
    server_url: &str,
    pages: &[PageEntry],
    idx: usize,
    live: &LiveExperiment,
    cache: &mut HashMap<String, EncodedImage>,
) -> Option<PagePrediction> {
    let center_img = encode_page(&pages[idx], live.prep, ImageRole::Center, cache);
    let body = serde_json::json!({
        "model": "test",
        "messages": [{
            "role": "user",
            "content": [
                { "type": "image_url", "image_url": { "url": format!("data:{};base64,{}", center_img.mime, center_img.b64) } },
                { "type": "text", "text": build_semantic_prompt() }
            ]
        }],
        "max_tokens": 96,
        "temperature": 0.0
    });
    let json = post_json_with_retry(http, server_url, &body).await?;
    let traits = parse_semantic_traits(&extract_content(&json).unwrap_or_default())?;
    let (mood, intensity) = map_traits_to_mood(&traits);
    Some(PagePrediction {
        mood,
        intensity,
        winner_score: 0.0,
        runner_up_score: 0.0,
        note: Some("semantic".to_string()),
    })
}

#[derive(Default)]
struct SemanticTraits {
    awe: u8,
    urgency: u8,
    grief: u8,
    levity: u8,
    intimacy: u8,
    dread: u8,
    calm: u8,
    enigma: u8,
}

fn parse_semantic_traits(text: &str) -> Option<SemanticTraits> {
    let re = regex::Regex::new(
        r"(?i)AWE=(\d)\s+URGENCY=(\d)\s+GRIEF=(\d)\s+LEVITY=(\d)\s+INTIMACY=(\d)\s+DREAD=(\d)\s+CALM=(\d)\s+ENIGMA=(\d)",
    )
    .unwrap();
    let caps = re.captures(text.trim())?;
    Some(SemanticTraits {
        awe: caps.get(1)?.as_str().parse().ok()?,
        urgency: caps.get(2)?.as_str().parse().ok()?,
        grief: caps.get(3)?.as_str().parse().ok()?,
        levity: caps.get(4)?.as_str().parse().ok()?,
        intimacy: caps.get(5)?.as_str().parse().ok()?,
        dread: caps.get(6)?.as_str().parse().ok()?,
        calm: caps.get(7)?.as_str().parse().ok()?,
        enigma: caps.get(8)?.as_str().parse().ok()?,
    })
}

fn map_traits_to_mood(traits: &SemanticTraits) -> (String, u8) {
    let candidates = [
        ("sadness", traits.grief),
        ("comedy", traits.levity),
        ("romance", traits.intimacy),
        ("horror", traits.dread),
        ("peaceful", traits.calm),
        ("mystery", traits.enigma),
    ];

    let (best_label, best_value) = candidates
        .iter()
        .max_by_key(|(_, value)| *value)
        .map(|(label, value)| (*label, *value))
        .unwrap();

    if best_value >= traits.awe.max(traits.urgency) {
        return (best_label.to_string(), best_value.max(1));
    }

    if traits.awe >= traits.urgency {
        ("epic".to_string(), traits.awe.max(1))
    } else {
        ("tension".to_string(), traits.urgency.max(1))
    }
}

fn load_cached_summary(
    spec: &ExperimentSpec,
    chapters: &BTreeMap<String, Vec<PageEntry>>,
    cache_path: &Path,
) -> ExperimentSummary {
    let result = std::fs::read_to_string(cache_path)
        .ok()
        .and_then(|text| serde_json::from_str::<HashMap<String, String>>(&text).ok());
    let Some(result_map) = result else {
        return ExperimentSummary {
            id: spec.id.to_string(),
            label: spec.label.to_string(),
            source: "cache".to_string(),
            status: "missing".to_string(),
            model: "Qwen3-VL-4B-Thinking".to_string(),
            prep: "legacy_672_jpeg".to_string(),
            decision: "majority".to_string(),
            sampling: "greedy_hist".to_string(),
            runtime: "historical".to_string(),
            strict: 0,
            relaxed: 0,
            intensity: 0,
            total: 0,
            errors: 0,
            second_pass_pages: 0,
            avg_window_s: 0.0,
            min_window_s: 0.0,
            max_window_s: 0.0,
            vram_mib: None,
            cache_path: None,
            notes: format!("Missing cache: {}", cache_path.display()),
            per_mood: BTreeMap::new(),
        };
    };

    build_summary_from_results(
        spec,
        "cache",
        "Qwen3-VL-4B-Thinking",
        "legacy_672_jpeg",
        "majority",
        "greedy_hist",
        "historical",
        0,
        0,
        &[],
        None,
        cache_path,
        spec.notes,
        chapters,
        &result_map,
    )
}

#[allow(clippy::too_many_arguments)]
fn build_summary_from_results(
    spec: &ExperimentSpec,
    source: &str,
    model: &str,
    prep: &str,
    decision: &str,
    sampling: &str,
    runtime: &str,
    errors: u32,
    second_pass_pages: u32,
    window_durations: &[f64],
    vram_mib: Option<u32>,
    cache_path: &Path,
    notes: &str,
    chapters: &BTreeMap<String, Vec<PageEntry>>,
    result_map: &HashMap<String, String>,
) -> ExperimentSummary {
    let mut strict = 0u32;
    let mut relaxed = 0u32;
    let mut intensity = 0u32;
    let mut total = 0u32;
    let mut per_mood = BTreeMap::new();

    for pages in chapters.values() {
        for page in pages {
            if page.confidence <= 0.0 {
                continue;
            }
            total += 1;
            let entry = per_mood.entry(page.mood.clone()).or_insert_with(MoodBreakdown::default);
            entry.total += 1;
            if let Some(tag) = result_map.get(&page.rel_path) {
                let mut parts = tag.split(':');
                let got_mood = parts.next().unwrap_or("???");
                let got_intensity = parts
                    .next()
                    .and_then(|s| s.parse::<u8>().ok())
                    .unwrap_or(0);
                let strict_match = got_mood == page.mood;
                let relaxed_match = strict_match || is_relaxed_match(got_mood, &page.mood);
                if strict_match {
                    strict += 1;
                    entry.strict += 1;
                }
                if relaxed_match {
                    relaxed += 1;
                    entry.relaxed += 1;
                }
                if got_intensity == page.intensity {
                    intensity += 1;
                }
            }
        }
    }

    let avg_window_s = if window_durations.is_empty() {
        0.0
    } else {
        window_durations.iter().sum::<f64>() / window_durations.len() as f64
    };
    let min_window_s = window_durations
        .iter()
        .cloned()
        .fold(f64::INFINITY, f64::min);
    let max_window_s = window_durations
        .iter()
        .cloned()
        .fold(f64::NEG_INFINITY, f64::max);

    ExperimentSummary {
        id: spec.id.to_string(),
        label: spec.label.to_string(),
        source: source.to_string(),
        status: "ok".to_string(),
        model: model.to_string(),
        prep: prep.to_string(),
        decision: decision.to_string(),
        sampling: sampling.to_string(),
        runtime: runtime.to_string(),
        strict,
        relaxed,
        intensity,
        total,
        errors,
        second_pass_pages,
        avg_window_s,
        min_window_s: if min_window_s.is_finite() { min_window_s } else { 0.0 },
        max_window_s: if max_window_s.is_finite() { max_window_s } else { 0.0 },
        vram_mib,
        cache_path: Some(cache_path.display().to_string()),
        notes: notes.to_string(),
        per_mood,
    }
}

fn is_relaxed_match(detected: &str, expected: &str) -> bool {
    let family = |m: &str| -> u8 {
        match m {
            "epic" | "tension" => 1,
            "sadness" | "peaceful" => 2,
            "comedy" | "romance" => 3,
            "horror" | "mystery" => 4,
            _ => 255,
        }
    };
    family(detected) == family(expected)
}

fn print_experiment_summary(summary: &ExperimentSummary) {
    match summary.status.as_str() {
        "ok" => {
            let strict_pct = pct(summary.strict, summary.total);
            let relaxed_pct = pct(summary.relaxed, summary.total);
            println!(
                "    {DIM}{} — strict {}/{} ({:.1}%), relaxed {}/{} ({:.1}%), intensity {}/{}, avg {:.1}s, second-pass {}{RESET}",
                summary.id,
                summary.strict,
                summary.total,
                strict_pct,
                summary.relaxed,
                summary.total,
                relaxed_pct,
                summary.intensity,
                summary.total,
                summary.avg_window_s,
                summary.second_pass_pages
            );
        }
        _ => {
            println!(
                "    {YELLOW}{} — {}: {}{RESET}",
                summary.id, summary.status, summary.notes
            );
        }
    }
}

fn print_comparison_table(summaries: &[ExperimentSummary], baseline: &ExperimentSummary) {
    let mut ordered = summaries.to_vec();
    ordered.sort_by(|a, b| {
        if a.id == baseline.id {
            return std::cmp::Ordering::Less;
        }
        if b.id == baseline.id {
            return std::cmp::Ordering::Greater;
        }
        b.strict.cmp(&a.strict)
    });

    println!("\n  {BOLD}Comparative Table{RESET}");
    println!(
        "  {DIM}| {:<18} | {:<16} | {:<16} | {:>7} | {:>7} | {:>7} | {:>7} | {:>7} | {:>6} |{RESET}",
        "experiment", "model", "decision", "strict", "d_str", "relax", "d_rel", "int", "avg_s"
    );

    for summary in ordered {
        let delta_strict = summary.strict as i32 - baseline.strict as i32;
        let delta_relaxed = summary.relaxed as i32 - baseline.relaxed as i32;
        let strict_color = if delta_strict > 0 { GREEN } else if delta_strict < 0 { RED } else { CYAN };
        let relax_color = if delta_relaxed > 0 { GREEN } else if delta_relaxed < 0 { RED } else { CYAN };
        println!(
            "  | {:<18} | {:<16} | {:<16} | {:>3}/{:<3} | {strict_color}{:>+4}{RESET}   | {:>3}/{:<3} | {relax_color}{:>+4}{RESET}   | {:>3}/{:<3} | {:>6.1} |",
            summary.id,
            truncate(&summary.model, 16),
            truncate(&summary.decision, 16),
            summary.strict,
            summary.total,
            delta_strict,
            summary.relaxed,
            summary.total,
            delta_relaxed,
            summary.intensity,
            summary.total,
            summary.avg_window_s
        );
    }

    println!("\n  {BOLD}Baseline loaded from cache:{RESET} {}", baseline.id);
}

fn save_suite_summary(
    results_dir: &Path,
    realtest_dir: &Path,
    summaries: &[ExperimentSummary],
    baseline: &ExperimentSummary,
) {
    let slug = std::env::var("REALTEST_FILTER")
        .ok()
        .map(|f| sanitize_slug(&f))
        .unwrap_or_else(|| "all".to_string());
    let page_suffix = std::env::var("REALTEST_PAGE_LIMIT")
        .ok()
        .map(|v| format!("_first{}", v))
        .unwrap_or_default();
    let json_path = results_dir.join(format!("realtest_suite_{}{}.json", slug, page_suffix));
    let md_path = results_dir.join(format!("realtest_suite_{}{}.md", slug, page_suffix));

    let payload = serde_json::json!({
        "baseline_id": baseline.id,
        "baseline_strict": baseline.strict,
        "baseline_relaxed": baseline.relaxed,
        "baseline_intensity": baseline.intensity,
        "results_dir": results_dir,
        "realtest_dir": realtest_dir,
        "summaries": summaries,
    });
    let _ = std::fs::write(&json_path, serde_json::to_string_pretty(&payload).unwrap());

    let mut markdown = String::new();
    markdown.push_str("| experiment | model | prep | decision | strict | delta_strict | relaxed | delta_relaxed | intensity | avg_window_s |\n");
    markdown.push_str("|---|---|---|---|---:|---:|---:|---:|---:|---:|\n");
    for summary in summaries {
        markdown.push_str(&format!(
            "| {} | {} | {} | {} | {}/{} | {:+} | {}/{} | {:+} | {}/{} | {:.1} |\n",
            summary.id,
            summary.model,
            summary.prep,
            summary.decision,
            summary.strict,
            summary.total,
            summary.strict as i32 - baseline.strict as i32,
            summary.relaxed,
            summary.total,
            summary.relaxed as i32 - baseline.relaxed as i32,
            summary.intensity,
            summary.total,
            summary.avg_window_s
        ));
    }
    let _ = std::fs::write(&md_path, markdown);

    println!(
        "\n  {DIM}Suite summary saved to:{} {}{RESET}",
        json_path.display(),
        md_path.display()
    );
}

fn pct(value: u32, total: u32) -> f64 {
    if total == 0 {
        0.0
    } else {
        value as f64 / total as f64 * 100.0
    }
}

fn truncate(value: &str, max_len: usize) -> String {
    let mut chars = value.chars();
    let truncated = chars.by_ref().take(max_len).collect::<String>();
    if chars.next().is_some() {
        truncated
    } else {
        value.to_string()
    }
}

fn sanitize_slug(value: &str) -> String {
    value.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '_' })
        .collect::<String>()
        .trim_matches('_')
        .to_string()
}

fn query_gpu_memory_used_mib() -> Option<u32> {
    let output = std::process::Command::new("nvidia-smi")
        .args(["--query-gpu=memory.used", "--format=csv,noheader,nounits"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    text.lines().next()?.trim().parse::<u32>().ok()
}
