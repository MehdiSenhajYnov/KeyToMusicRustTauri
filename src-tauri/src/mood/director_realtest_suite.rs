use crate::mood::inference::{
    self, extract_content, LlamaRuntimeIntent, LlamaServer, LlamaServerStartOptions,
};
use crate::mood::shared::MoodScores;
use crate::mood::soundtrack_commit::{SoundtrackCommitter, SoundtrackObservation};
use crate::mood::winner;
use crate::types::{BaseMood, MoodIntensity};
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

const TRANSITION_MATRIX: [[f32; 8]; 8] = [
    [1.0, 0.7, 0.5, 0.2, 0.2, 0.4, 0.3, 0.3],
    [0.9, 1.0, 0.5, 0.3, 0.3, 0.7, 0.4, 0.8],
    [0.3, 0.5, 1.0, 0.2, 0.6, 0.3, 0.7, 0.4],
    [0.3, 0.3, 0.3, 1.0, 0.7, 0.2, 0.8, 0.3],
    [0.2, 0.3, 0.6, 0.7, 1.0, 0.2, 0.8, 0.3],
    [0.4, 0.8, 0.5, 0.1, 0.1, 1.0, 0.3, 0.7],
    [0.4, 0.4, 0.6, 0.8, 0.7, 0.3, 1.0, 0.5],
    [0.4, 0.8, 0.4, 0.3, 0.3, 0.7, 0.4, 1.0],
];

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
        let clamp =
            |idx: isize| -> usize { idx.clamp(0, total.saturating_sub(1) as isize) as usize };
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
    Wide5SelectiveRepairHold,
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
            Self::Wide5SelectiveRepairHold => "wide5_selective_repair_hold",
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
    gpu_layers: Option<u32>,
}

#[derive(Clone, Debug)]
struct ModelCandidate {
    key: &'static str,
    label: &'static str,
    quantization: &'static str,
    weights_mib: Option<u64>,
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

type PageVoteStats = winner::VoteStats;

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
struct PageResult {
    rel_path: String,
    chapter: String,
    page_number: u32,
    expected_mood: String,
    expected_intensity: u8,
    predicted_mood: String,
    predicted_intensity: u8,
    verdict: String,
    intensity_match: bool,
}

#[derive(Clone, Debug, Serialize)]
struct ExperimentSummary {
    id: String,
    label: String,
    source: String,
    status: String,
    model: String,
    quantization: String,
    prep: String,
    decision: String,
    sampling: String,
    runtime: String,
    runtime_context_size: Option<u32>,
    runtime_parallel_slots: Option<u32>,
    runtime_gpu_layers: Option<u32>,
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
    weights_mib: Option<u64>,
    cache_path: Option<String>,
    notes: String,
    per_mood: BTreeMap<String, MoodBreakdown>,
    page_results: Vec<PageResult>,
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
    let strict_prod_timing = strict_prod_timing_enabled();

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
        "  {DIM}Default mode now runs the comparison suite. Use REALTEST_EXPERIMENTS=id1,id2, REALTEST_EXPERIMENTS=current_runtime_showdown, or REALTEST_EXPERIMENTS=current_method_showdown to narrow it, and REALTEST_PAGE_LIMIT=N for a smoke run.{RESET}\n"
    );
    if strict_prod_timing {
        println!(
            "  {YELLOW}Strict prod timing:{RESET} {DIM}image prep + request path measured, benchmark image/reprompt caches disabled.{RESET}\n"
        );
    }

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
                let summary =
                    run_live_experiment(&spec, live, &models, &chapters, &results_dir, verbose)
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
    print_page_comparison(&summaries, &chapters);
    save_suite_summary(
        &results_dir,
        &realtest_dir,
        &summaries,
        &baseline,
        &chapters,
    );
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
        if let Err(err) = image::load_from_memory(&raw_bytes) {
            tracing::warn!("Skipping unreadable RealTest page {}: {}", rel_path, err);
            skipped += 1;
            continue;
        }
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

fn env_truthy(key: &str) -> bool {
    std::env::var(key)
        .ok()
        .map(|value| matches!(value.trim(), "1" | "true" | "yes" | "on"))
        .unwrap_or(false)
}

fn strict_prod_timing_enabled() -> bool {
    env_truthy("REALTEST_STRICT_PROD_TIMING")
}

fn file_size_bytes(path: &Path) -> Option<u64> {
    std::fs::metadata(path).ok().map(|meta| meta.len())
}

fn combined_file_size_mib(model_path: &Path, mmproj_path: &Path) -> Option<u64> {
    let total_bytes = file_size_bytes(model_path)? + file_size_bytes(mmproj_path)?;
    Some(total_bytes.div_ceil(1024 * 1024))
}

fn push_unique_selection(selected: &mut Vec<String>, ids: impl IntoIterator<Item = &'static str>) {
    for id in ids {
        if !selected.iter().any(|value| value == id) {
            selected.push(id.to_string());
        }
    }
}

fn selection_output_slug() -> Option<String> {
    if let Some(explicit) = std::env::var("REALTEST_OUTPUT_SLUG")
        .ok()
        .map(|value| sanitize_slug(&value))
        .filter(|value| !value.is_empty())
    {
        return Some(explicit);
    }

    let raw = std::env::var("REALTEST_EXPERIMENTS").ok()?;
    let values = raw
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();

    if values.contains(&"current_quantization_showdown") {
        return Some("current_quantization_showdown".to_string());
    }
    if values.contains(&"current_runtime_showdown") {
        return Some("current_runtime_showdown".to_string());
    }

    None
}

fn requested_runtime_label(runtime: RuntimeProfile) -> String {
    let gpu_layers = runtime
        .gpu_layers
        .map(|value| value.to_string())
        .unwrap_or_else(|| "auto".to_string());
    format!(
        "{} [c={} np={} ngl={}]",
        runtime.name, runtime.context_size, runtime.parallel_slots, gpu_layers
    )
}

fn resolved_runtime_label(
    name: &str,
    context_size: u32,
    parallel_slots: u32,
    gpu_layers: u32,
) -> String {
    format!(
        "{} [c={} np={} ngl={}]",
        name, context_size, parallel_slots, gpu_layers
    )
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
        gpu_layers: None,
    };
    const RUNTIME_QUIET: RuntimeProfile = RuntimeProfile {
        name: "quiet_8192x1",
        intent: LlamaRuntimeIntent::BenchmarkPrimary,
        context_size: 8192,
        parallel_slots: 1,
        gpu_layers: None,
    };
    const RUNTIME_PROD_CURRENT: RuntimeProfile = RuntimeProfile {
        name: "prod_8192x1",
        intent: LlamaRuntimeIntent::BenchmarkPrimary,
        context_size: 8192,
        parallel_slots: 1,
        gpu_layers: None,
    };
    const RUNTIME_CONTEXT_16384: RuntimeProfile = RuntimeProfile {
        name: "ctx_16384x1",
        intent: LlamaRuntimeIntent::BenchmarkPrimary,
        context_size: 16384,
        parallel_slots: 1,
        gpu_layers: None,
    };
    const RUNTIME_PARALLEL_2: RuntimeProfile = RuntimeProfile {
        name: "parallel_12288x2",
        intent: LlamaRuntimeIntent::BenchmarkPrimary,
        context_size: 12288,
        parallel_slots: 2,
        gpu_layers: None,
    };
    const RUNTIME_GPU_32: RuntimeProfile = RuntimeProfile {
        name: "gpu_12288x1_ngl32",
        intent: LlamaRuntimeIntent::BenchmarkPrimary,
        context_size: 12288,
        parallel_slots: 1,
        gpu_layers: Some(32),
    };
    const RUNTIME_GPU_64: RuntimeProfile = RuntimeProfile {
        name: "gpu_12288x1_ngl64",
        intent: LlamaRuntimeIntent::BenchmarkPrimary,
        context_size: 12288,
        parallel_slots: 1,
        gpu_layers: Some(64),
    };
    const RUNTIME_RESEARCH: RuntimeProfile = RuntimeProfile {
        name: "research_32768x4",
        intent: LlamaRuntimeIntent::ResearchLarge,
        context_size: 32768,
        parallel_slots: 4,
        gpu_layers: None,
    };

    let current_method_runtime =
        |id: &'static str, label: &'static str, notes: &'static str, runtime: RuntimeProfile| {
            ExperimentSpec {
                id,
                label,
                notes,
                mode: ExperimentMode::Live(LiveExperiment {
                    model_key: "thinking4b",
                    prep: ImagePrepMode::Legacy672Jpeg,
                    context: ContextWindow::Wide5,
                    prompt_style: PromptStyle::WideSequenceWindow,
                    sampling: GREEDY_HIST,
                    runtime,
                    grammar: false,
                    decision: DecisionStrategy::Wide5SelectiveRepairHold,
                }),
            }
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
            id: "production_cache",
            label: "Current production cache",
            notes: "Current production cumulative result map loaded from cache, no rerun.",
            mode: ExperimentMode::Cached {
                cache_filename: "realtests/realtest_suite_t4b_wide5_selective_hold.json",
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
        current_method_runtime(
            "t4b_wide5_selective_hold",
            "Qwen3-VL-4B-Thinking wide-5 selective repair + action hold",
            "Wide-5 selective repairs plus a zero-cost delayed epic->tension switch when a short tension bridge is sandwiched between confirmed epic runs. This benchmark baseline is explicitly aligned with the current production runtime (`8192x1`, GPU layers auto).",
            RUNTIME_PROD_CURRENT,
        ),
        current_method_runtime(
            "t4b_wide5_selective_hold_runtime_prod_8192",
            "Qwen3-VL-4B-Thinking current-runtime showdown: prod 8192x1",
            "Exact current winner method on the app-default BenchmarkPrimary runtime sizing (`8192x1`, GPU layers auto). Kept as an explicit prod-runtime alias even though the main benchmark baseline is now aligned to the same runtime.",
            RUNTIME_PROD_CURRENT,
        ),
        current_method_runtime(
            "t4b_wide5_selective_hold_runtime_legacy_12288",
            "Qwen3-VL-4B-Thinking current-runtime showdown: legacy benchmark 12288x1",
            "Exact current winner method on the pre-alignment benchmark runtime (`12288x1`, GPU layers auto). Kept as a legacy reference after aligning the main benchmark baseline to production.",
            RUNTIME_DEFAULT,
        ),
        current_method_runtime(
            "t4b_wide5_selective_hold_runtime_ctx_16384",
            "Qwen3-VL-4B-Thinking current-runtime showdown: larger context 16384x1",
            "Exact current winner method, same model and prompt stack, but with a larger single-slot context budget to test whether wide-5 gains anything from extra KV headroom.",
            RUNTIME_CONTEXT_16384,
        ),
        current_method_runtime(
            "t4b_wide5_selective_hold_runtime_parallel_2",
            "Qwen3-VL-4B-Thinking current-runtime showdown: parallel 12288x2",
            "Exact current winner method, same model and prompt stack, but with two llama-server slots to measure any stability/latency/VRAM tradeoff on a sequential benchmark workload.",
            RUNTIME_PARALLEL_2,
        ),
        current_method_runtime(
            "t4b_wide5_selective_hold_runtime_gpu_32",
            "Qwen3-VL-4B-Thinking current-runtime showdown: lower GPU offload ngl32",
            "Exact current winner method, same model and prompt stack, but with an explicit lower GPU offload (`-ngl 32`) to approximate a weaker-memory profile.",
            RUNTIME_GPU_32,
        ),
        current_method_runtime(
            "t4b_wide5_selective_hold_runtime_gpu_64",
            "Qwen3-VL-4B-Thinking current-runtime showdown: higher GPU offload ngl64",
            "Exact current winner method, same model and prompt stack, but with a more aggressive explicit GPU offload (`-ngl 64`) to test whether the RTX 4070 stack benefits from pushing more layers on GPU than the current auto heuristic.",
            RUNTIME_GPU_64,
        ),
        ExperimentSpec {
            id: "t4b_wide5_selective_hold_mmproj_q8_0",
            label: "Qwen3-VL-4B-Thinking current-method quant showdown: Q4_K_M + mmproj Q8_0",
            notes: "Current production methodology applied unchanged to the official Qwen3-VL-4B-Thinking GGUF files, keeping the LLM at Q4_K_M while quantizing the official mmproj from F16 to Q8_0. This is the only lighter official stack variant around the current baseline.",
            mode: ExperimentMode::Live(LiveExperiment {
                model_key: "thinking4b_mmproj_q8_0",
                prep: ImagePrepMode::Legacy672Jpeg,
                context: ContextWindow::Wide5,
                prompt_style: PromptStyle::WideSequenceWindow,
                sampling: GREEDY_HIST,
                runtime: RUNTIME_PROD_CURRENT,
                grammar: false,
                decision: DecisionStrategy::Wide5SelectiveRepairHold,
            }),
        },
        ExperimentSpec {
            id: "t4b_wide5_selective_hold_q8_0",
            label: "Qwen3-VL-4B-Thinking current-method quant showdown: Q8_0",
            notes: "Current production methodology applied unchanged to the official Qwen3-VL-4B-Thinking Q8_0 GGUF, with the same prompt backbone, preprocessing, selective repairs, and action bridge hold as production.",
            mode: ExperimentMode::Live(LiveExperiment {
                model_key: "thinking4b_q8_0",
                prep: ImagePrepMode::Legacy672Jpeg,
                context: ContextWindow::Wide5,
                prompt_style: PromptStyle::WideSequenceWindow,
                sampling: GREEDY_HIST,
                runtime: RUNTIME_PROD_CURRENT,
                grammar: false,
                decision: DecisionStrategy::Wide5SelectiveRepairHold,
            }),
        },
        ExperimentSpec {
            id: "t4b_wide5_selective_hold_f16",
            label: "Qwen3-VL-4B-Thinking current-method quant showdown: F16",
            notes: "Current production methodology applied unchanged to the official Qwen3-VL-4B-Thinking F16 GGUF, with the same prompt backbone, preprocessing, selective repairs, and action bridge hold as production.",
            mode: ExperimentMode::Live(LiveExperiment {
                model_key: "thinking4b_f16",
                prep: ImagePrepMode::Legacy672Jpeg,
                context: ContextWindow::Wide5,
                prompt_style: PromptStyle::WideSequenceWindow,
                sampling: GREEDY_HIST,
                runtime: RUNTIME_PROD_CURRENT,
                grammar: false,
                decision: DecisionStrategy::Wide5SelectiveRepairHold,
            }),
        },
        ExperimentSpec {
            id: "q35_9b_wide5_selective_hold",
            label: "Qwen3.5-9B current-method showdown",
            notes: "Current production methodology applied unchanged to Qwen3.5-9B: same wide-5 prompt backbone, same legacy 672 JPEG prep, same selective repairs, same action bridge hold.",
            mode: ExperimentMode::Live(LiveExperiment {
                model_key: "qwen35_9b",
                prep: ImagePrepMode::Legacy672Jpeg,
                context: ContextWindow::Wide5,
                prompt_style: PromptStyle::WideSequenceWindow,
                sampling: GREEDY_HIST,
                runtime: RUNTIME_PROD_CURRENT,
                grammar: false,
                decision: DecisionStrategy::Wide5SelectiveRepairHold,
            }),
        },
        ExperimentSpec {
            id: "q35_4b_wide5_selective_hold",
            label: "Qwen3.5-4B current-method showdown",
            notes: "Current production methodology applied unchanged to Qwen3.5-4B: same wide-5 prompt backbone, same legacy 672 JPEG prep, same selective repairs, same action bridge hold.",
            mode: ExperimentMode::Live(LiveExperiment {
                model_key: "qwen35_4b",
                prep: ImagePrepMode::Legacy672Jpeg,
                context: ContextWindow::Wide5,
                prompt_style: PromptStyle::WideSequenceWindow,
                sampling: GREEDY_HIST,
                runtime: RUNTIME_PROD_CURRENT,
                grammar: false,
                decision: DecisionStrategy::Wide5SelectiveRepairHold,
            }),
        },
        ExperimentSpec {
            id: "q35_4b_opus_v2_wide5_selective_hold",
            label: "Qwen3.5-4B Opus-distilled v2 current-method showdown",
            notes: "Current production methodology applied unchanged to Jackrong's Qwen3.5-4B Claude-4.6-Opus distilled v2 GGUF.",
            mode: ExperimentMode::Live(LiveExperiment {
                model_key: "q35_4b_opus_v2",
                prep: ImagePrepMode::Legacy672Jpeg,
                context: ContextWindow::Wide5,
                prompt_style: PromptStyle::WideSequenceWindow,
                sampling: GREEDY_HIST,
                runtime: RUNTIME_PROD_CURRENT,
                grammar: false,
                decision: DecisionStrategy::Wide5SelectiveRepairHold,
            }),
        },
        ExperimentSpec {
            id: "gemma4_e4b_wide5_selective_hold",
            label: "Gemma 4 E4B current-method showdown",
            notes: "Current production methodology applied unchanged to Gemma 4 E4B: same wide-5 prompt backbone, same legacy 672 JPEG prep, same selective repairs, same action bridge hold.",
            mode: ExperimentMode::Live(LiveExperiment {
                model_key: "gemma4_e4b",
                prep: ImagePrepMode::Legacy672Jpeg,
                context: ContextWindow::Wide5,
                prompt_style: PromptStyle::WideSequenceWindow,
                sampling: GREEDY_HIST,
                runtime: RUNTIME_PROD_CURRENT,
                grammar: false,
                decision: DecisionStrategy::Wide5SelectiveRepairHold,
            }),
        },
        ExperimentSpec {
            id: "gemma4_e2b_wide5_selective_hold",
            label: "Gemma 4 E2B current-method showdown",
            notes: "Current production methodology applied unchanged to Gemma 4 E2B: same wide-5 prompt backbone, same legacy 672 JPEG prep, same selective repairs, same action bridge hold.",
            mode: ExperimentMode::Live(LiveExperiment {
                model_key: "gemma4_e2b",
                prep: ImagePrepMode::Legacy672Jpeg,
                context: ContextWindow::Wide5,
                prompt_style: PromptStyle::WideSequenceWindow,
                sampling: GREEDY_HIST,
                runtime: RUNTIME_PROD_CURRENT,
                grammar: false,
                decision: DecisionStrategy::Wide5SelectiveRepairHold,
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
    let mut selected: Vec<String> = raw
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    if selected
        .iter()
        .any(|value| value == "current_method_showdown")
    {
        selected.retain(|value| value != "current_method_showdown");
        push_unique_selection(
            &mut selected,
            [
                "t4b_wide5_selective_hold",
                "q35_9b_wide5_selective_hold",
                "q35_4b_wide5_selective_hold",
                "q35_4b_opus_v2_wide5_selective_hold",
            ],
        );
    }
    if selected.iter().any(|value| value == "gemma4_showdown") {
        selected.retain(|value| value != "gemma4_showdown");
        push_unique_selection(
            &mut selected,
            [
                "t4b_wide5_selective_hold",
                "gemma4_e4b_wide5_selective_hold",
                "gemma4_e2b_wide5_selective_hold",
            ],
        );
    }
    if selected
        .iter()
        .any(|value| value == "current_runtime_showdown")
    {
        selected.retain(|value| value != "current_runtime_showdown");
        push_unique_selection(
            &mut selected,
            [
                "t4b_wide5_selective_hold",
                "t4b_wide5_selective_hold_runtime_legacy_12288",
                "t4b_wide5_selective_hold_runtime_ctx_16384",
                "t4b_wide5_selective_hold_runtime_parallel_2",
                "t4b_wide5_selective_hold_runtime_gpu_32",
                "t4b_wide5_selective_hold_runtime_gpu_64",
            ],
        );
    }
    if selected
        .iter()
        .any(|value| value == "current_quantization_showdown")
    {
        selected.retain(|value| value != "current_quantization_showdown");
        push_unique_selection(
            &mut selected,
            [
                "t4b_wide5_selective_hold",
                "t4b_wide5_selective_hold_mmproj_q8_0",
                "t4b_wide5_selective_hold_q8_0",
                "t4b_wide5_selective_hold_f16",
            ],
        );
    }
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
            "llm=Q4_K_M + mmproj=F16",
            models_dir.join("Qwen3-VL-4B-Thinking/Qwen3VL-4B-Thinking-Q4_K_M.gguf"),
            models_dir.join("Qwen3-VL-4B-Thinking/mmproj-Qwen3VL-4B-Thinking-F16.gguf"),
            Some("none"),
        ),
        (
            "thinking4b_mmproj_q8_0",
            "Qwen3-VL-4B-Thinking",
            "llm=Q4_K_M + mmproj=Q8_0",
            models_dir.join("Qwen3-VL-4B-Thinking/Qwen3VL-4B-Thinking-Q4_K_M.gguf"),
            models_dir.join("Qwen3-VL-4B-Thinking/mmproj-Qwen3VL-4B-Thinking-Q8_0.gguf"),
            Some("none"),
        ),
        (
            "thinking4b_q8_0",
            "Qwen3-VL-4B-Thinking",
            "llm=Q8_0 + mmproj=F16",
            models_dir.join("Qwen3-VL-4B-Thinking/Qwen3VL-4B-Thinking-Q8_0.gguf"),
            models_dir.join("Qwen3-VL-4B-Thinking/mmproj-Qwen3VL-4B-Thinking-F16.gguf"),
            Some("none"),
        ),
        (
            "thinking4b_f16",
            "Qwen3-VL-4B-Thinking",
            "llm=F16 + mmproj=F16",
            models_dir.join("Qwen3-VL-4B-Thinking/Qwen3VL-4B-Thinking-F16.gguf"),
            models_dir.join("Qwen3-VL-4B-Thinking/mmproj-Qwen3VL-4B-Thinking-F16.gguf"),
            Some("none"),
        ),
        (
            "qwen35_4b",
            "Qwen3.5-4B",
            "llm=Q4_K_M + mmproj=F16",
            models_dir.join("unsloth_Qwen3.5-4B-GGUF/Qwen3.5-4B-Q4_K_M.gguf"),
            models_dir.join("unsloth_Qwen3.5-4B-GGUF/mmproj-F16.gguf"),
            Some("none"),
        ),
        (
            "qwen35_9b",
            "Qwen3.5-9B",
            "llm=Q4_K_M + mmproj=F16",
            models_dir.join("unsloth_Qwen3.5-9B-GGUF/Qwen3.5-9B-Q4_K_M.gguf"),
            models_dir.join("unsloth_Qwen3.5-9B-GGUF/mmproj-F16.gguf"),
            Some("none"),
        ),
        (
            "qwen3vl_2b",
            "Qwen3-VL-2B-Instruct",
            "llm=Q4_K_M + mmproj=F16",
            models_dir.join("Qwen_Qwen3-VL-2B-Instruct-GGUF/Qwen3VL-2B-Instruct-Q4_K_M.gguf"),
            models_dir.join("Qwen_Qwen3-VL-2B-Instruct-GGUF/mmproj-Qwen3VL-2B-Instruct-F16.gguf"),
            Some("none"),
        ),
    ];

    for (key, label, quantization, model_path, mmproj_path, reasoning_format) in known {
        if model_path.exists() && mmproj_path.exists() {
            models.insert(
                key,
                ModelCandidate {
                    key,
                    label,
                    quantization,
                    weights_mib: combined_file_size_mib(&model_path, &mmproj_path),
                    model_path,
                    mmproj_path,
                    reasoning_format,
                },
            );
        }
    }

    if let Some(thinking4b) = models.get("thinking4b").cloned() {
        models.insert(winner::WINNER_MODEL_KEY, thinking4b);
    }

    maybe_insert_known_model(
        &mut models,
        "q35_4b_opus_v2",
        "Qwen3.5-4B Claude-4.6-Opus Distilled v2",
        "llm=Q4_K_M + mmproj=BF16/F16",
        &[
            models_dir.join(
                "Jackrong_Qwen3.5-4B-Claude-4.6-Opus-Reasoning-Distilled-v2-GGUF/Qwen3.5-4B.Q4_K_M.gguf",
            ),
            models_dir.join(
                "Jackrong_Qwen3.5-4B-Claude-4.6-Opus-Reasoning-Distilled-v2-GGUF/Qwen3.5-4B-Q4_K_M.gguf",
            ),
        ],
        &[
            models_dir.join(
                "Jackrong_Qwen3.5-4B-Claude-4.6-Opus-Reasoning-Distilled-v2-GGUF/mmproj-BF16.gguf",
            ),
            models_dir.join(
                "Jackrong_Qwen3.5-4B-Claude-4.6-Opus-Reasoning-Distilled-v2-GGUF/mmproj-F16.gguf",
            ),
        ],
        Some("none"),
    );
    maybe_insert_known_model(
        &mut models,
        "gemma4_e4b",
        "Gemma 4 E4B",
        "llm=Q4_K_M + mmproj=F16",
        &[
            models_dir.join("unsloth_gemma-4-E4B-it-GGUF/gemma-4-E4B-it-Q4_K_M.gguf"),
            models_dir.join("gemma-4-E4B-it-GGUF/gemma-4-E4B-it-Q4_K_M.gguf"),
            models_dir.join("Gemma-4-E4B-it-GGUF/gemma-4-E4B-it-Q4_K_M.gguf"),
        ],
        &[
            models_dir.join("unsloth_gemma-4-E4B-it-GGUF/mmproj-F16.gguf"),
            models_dir.join("gemma-4-E4B-it-GGUF/mmproj-F16.gguf"),
            models_dir.join("Gemma-4-E4B-it-GGUF/mmproj-F16.gguf"),
        ],
        None,
    );
    maybe_insert_known_model(
        &mut models,
        "gemma4_e2b",
        "Gemma 4 E2B",
        "llm=Q4_K_M + mmproj=F16",
        &[
            models_dir.join("unsloth_gemma-4-E2B-it-GGUF/gemma-4-E2B-it-Q4_K_M.gguf"),
            models_dir.join("gemma-4-E2B-it-GGUF/gemma-4-E2B-it-Q4_K_M.gguf"),
            models_dir.join("Gemma-4-E2B-it-GGUF/gemma-4-E2B-it-Q4_K_M.gguf"),
        ],
        &[
            models_dir.join("unsloth_gemma-4-E2B-it-GGUF/mmproj-F16.gguf"),
            models_dir.join("gemma-4-E2B-it-GGUF/mmproj-F16.gguf"),
            models_dir.join("Gemma-4-E2B-it-GGUF/mmproj-F16.gguf"),
        ],
        None,
    );

    maybe_insert_env_model(
        &mut models,
        "glm41v_9b",
        "GLM-4.1V-9B-Thinking",
        "REALTEST_MODEL_GLM41V_9B",
        "REALTEST_MMPROJ_GLM41V_9B",
        "env",
        Some("none"),
    );
    maybe_insert_env_model(
        &mut models,
        "mangalmm",
        "MangaLMM",
        "REALTEST_MODEL_MANGALMM",
        "REALTEST_MMPROJ_MANGALMM",
        "env",
        Some("none"),
    );
    maybe_insert_env_model(
        &mut models,
        "q35_4b_opus_v2",
        "Qwen3.5-4B Claude-4.6-Opus Distilled v2",
        "REALTEST_MODEL_Q35_4B_OPUS_V2",
        "REALTEST_MMPROJ_Q35_4B_OPUS_V2",
        "env",
        Some("none"),
    );
    maybe_insert_env_model(
        &mut models,
        "gemma4_e4b",
        "Gemma 4 E4B",
        "REALTEST_MODEL_GEMMA4_E4B",
        "REALTEST_MMPROJ_GEMMA4_E4B",
        "env",
        None,
    );
    maybe_insert_env_model(
        &mut models,
        "gemma4_e2b",
        "Gemma 4 E2B",
        "REALTEST_MODEL_GEMMA4_E2B",
        "REALTEST_MMPROJ_GEMMA4_E2B",
        "env",
        None,
    );

    models
}

fn maybe_insert_known_model(
    models: &mut HashMap<&'static str, ModelCandidate>,
    key: &'static str,
    label: &'static str,
    quantization: &'static str,
    model_candidates: &[PathBuf],
    mmproj_candidates: &[PathBuf],
    reasoning_format: Option<&'static str>,
) {
    let Some(model_path) = model_candidates.iter().find(|path| path.exists()).cloned() else {
        return;
    };
    let Some(mmproj_path) = mmproj_candidates.iter().find(|path| path.exists()).cloned() else {
        return;
    };

    models.insert(
        key,
        ModelCandidate {
            key,
            label,
            quantization,
            weights_mib: combined_file_size_mib(&model_path, &mmproj_path),
            model_path,
            mmproj_path,
            reasoning_format,
        },
    );
}

fn maybe_insert_env_model(
    models: &mut HashMap<&'static str, ModelCandidate>,
    key: &'static str,
    label: &'static str,
    model_env: &str,
    mmproj_env: &str,
    quantization: &'static str,
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
                quantization,
                weights_mib: combined_file_size_mib(&model_path, &mmproj_path),
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
            quantization: "unavailable".to_string(),
            prep: live.prep.as_str().to_string(),
            decision: live.decision.as_str().to_string(),
            sampling: live.sampling.name.to_string(),
            runtime: requested_runtime_label(live.runtime),
            runtime_context_size: Some(live.runtime.context_size),
            runtime_parallel_slots: Some(live.runtime.parallel_slots),
            runtime_gpu_layers: live.runtime.gpu_layers,
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
            weights_mib: None,
            cache_path: None,
            notes: format!("Model '{}' not found locally", live.model_key),
            per_mood: BTreeMap::new(),
            page_results: Vec::new(),
        };
    };

    println!(
        "  {CYAN}{BOLD}▶ {}{RESET} {DIM}[{} | {} | {} | {} | {} | {} | {}]{RESET}",
        spec.label,
        model.label,
        model.quantization,
        live.prep.as_str(),
        live.context.as_str(),
        live.prompt_style.as_str(),
        live.decision.as_str(),
        requested_runtime_label(live.runtime)
    );

    let server_result = LlamaServer::start_with_options(
        model.model_path.to_str().unwrap(),
        model.mmproj_path.to_str().unwrap(),
        LlamaServerStartOptions {
            reasoning_format: model.reasoning_format.map(str::to_string),
            context_size: Some(live.runtime.context_size),
            parallel_slots: Some(live.runtime.parallel_slots),
            gpu_layers: live.runtime.gpu_layers,
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
                quantization: model.quantization.to_string(),
                prep: live.prep.as_str().to_string(),
                decision: live.decision.as_str().to_string(),
                sampling: live.sampling.name.to_string(),
                runtime: requested_runtime_label(live.runtime),
                runtime_context_size: Some(live.runtime.context_size),
                runtime_parallel_slots: Some(live.runtime.parallel_slots),
                runtime_gpu_layers: live.runtime.gpu_layers,
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
                weights_mib: model.weights_mib,
                cache_path: None,
                notes: format!("Server start failed: {err}"),
                per_mood: BTreeMap::new(),
                page_results: Vec::new(),
            };
        }
    };
    let (resolved_context_size, resolved_parallel_slots, resolved_gpu_layers) =
        server.resolved_runtime();

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
    let cache_path = results_dir.join(format!("realtest_suite_{}.json", spec.id));

    // Reset stale outputs before starting so interrupted reruns don't keep an old completed map.
    persist_result_map_snapshot(
        results_dir,
        spec.id,
        chapters,
        &cache_path,
        &result_map,
        true,
    );

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
            &mut result_map,
            &cache_path,
            verbose,
        )
        .await;
        total_errors += window_errors;
        window_durations.extend(window_results.iter().map(|w| w.elapsed_s));
        let winner_window_results = to_winner_window_predictions(&window_results);

        let vote_stats: Vec<PageVoteStats> = (0..pages.len())
            .map(|idx| winner::build_vote_stats(&winner_window_results, idx as u32))
            .collect();

        let mut predictions = match live.decision {
            DecisionStrategy::Majority => aggregate_majority(pages, &vote_stats),
            DecisionStrategy::CenterOverride => aggregate_center_override(pages, &vote_stats),
            DecisionStrategy::Direct => aggregate_direct(&window_results, pages.len()),
            DecisionStrategy::Wide5SelectiveRepair | DecisionStrategy::Wide5SelectiveRepairHold => {
                aggregate_majority(pages, &vote_stats)
            }
            DecisionStrategy::Viterbi
            | DecisionStrategy::FocusedReprompt
            | DecisionStrategy::OcrReprompt
            | DecisionStrategy::SemanticReprompt => aggregate_viterbi(&vote_stats),
        };

        if matches!(
            live.decision,
            DecisionStrategy::Wide5SelectiveRepair | DecisionStrategy::Wide5SelectiveRepairHold
        ) {
            let (repaired, extra_pages, extra_time) = apply_wide5_selective_repairs(
                &http,
                &server_url,
                pages,
                predictions,
                live,
                &mut encoding_cache,
                matches!(live.decision, DecisionStrategy::Wide5SelectiveRepairHold),
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
                        reprompt_with_ocr(&http, &server_url, pages, idx, live, &mut encoding_cache)
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

        // Replace provisional center-page checkpoints with finalized chapter predictions.
        persist_result_map_snapshot(
            results_dir,
            spec.id,
            chapters,
            &cache_path,
            &result_map,
            true,
        );
    }

    server.stop();
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    persist_result_map_snapshot(
        results_dir,
        spec.id,
        chapters,
        &cache_path,
        &result_map,
        true,
    );

    let effective_durations = if extra_time_total > 0.0 {
        let total_pages: usize = chapters.values().map(|pages| pages.len()).sum();
        let effective_avg =
            (window_durations.iter().sum::<f64>() + extra_time_total) / total_pages.max(1) as f64;
        vec![effective_avg; total_pages.max(1)]
    } else {
        window_durations.clone()
    };

    let mut summary = build_summary_from_results(
        spec,
        "live",
        model.label,
        model.quantization,
        live.prep.as_str(),
        live.decision.as_str(),
        live.sampling.name,
        &resolved_runtime_label(
            live.runtime.name,
            resolved_context_size,
            resolved_parallel_slots,
            resolved_gpu_layers,
        ),
        total_errors,
        second_pass_pages,
        &effective_durations,
        vram_mib,
        model.weights_mib,
        &cache_path,
        spec.notes,
        chapters,
        &result_map,
    );
    summary.runtime_context_size = Some(resolved_context_size);
    summary.runtime_parallel_slots = Some(resolved_parallel_slots);
    summary.runtime_gpu_layers = Some(resolved_gpu_layers);
    summary
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
                quantization: "mixed".to_string(),
                prep: "mixed".to_string(),
                decision: "weighted_viterbi".to_string(),
                sampling: "derived".to_string(),
                runtime: "sequential_ensemble".to_string(),
                runtime_context_size: None,
                runtime_parallel_slots: None,
                runtime_gpu_layers: None,
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
                weights_mib: None,
                cache_path: None,
                notes: format!("Missing dependency result map: {}", cache_path.display()),
                per_mood: BTreeMap::new(),
                page_results: Vec::new(),
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
    let _ = std::fs::write(
        &cache_path,
        serde_json::to_string_pretty(&result_map).unwrap(),
    );
    let _ = write_soundtrack_commit_companion(results_dir, spec.id, chapters, &result_map);

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
        "mixed",
        "weighted_viterbi",
        "derived",
        "sequential_ensemble",
        0,
        0,
        &durations,
        effective_vram,
        None,
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

fn build_soundtrack_commit_result_map(
    chapters: &BTreeMap<String, Vec<PageEntry>>,
    result_map: &HashMap<String, String>,
) -> HashMap<String, String> {
    let mut committer = SoundtrackCommitter::new();
    let mut committed_map = HashMap::new();

    for (chapter_key, pages) in chapters {
        for (idx, page) in pages.iter().enumerate() {
            let Some(tag) = result_map.get(&page.rel_path) else {
                continue;
            };
            let Some((mood_label, intensity_value)) = parse_tag(tag) else {
                continue;
            };
            let Some(mood) = BaseMood::from_str_opt(&mood_label) else {
                continue;
            };
            let intensity = MoodIntensity::from_u8(intensity_value);
            let future_context = pages
                .iter()
                .skip(idx + 1)
                .take(2)
                .filter_map(|next_page| {
                    let next_tag = result_map.get(&next_page.rel_path)?;
                    let (next_mood_label, next_intensity_value) = parse_tag(next_tag)?;
                    let next_mood = BaseMood::from_str_opt(&next_mood_label)?;
                    let next_intensity = MoodIntensity::from_u8(next_intensity_value);
                    Some(SoundtrackObservation {
                        page: Some(page_number_from_rel_path(&next_page.rel_path)),
                        mood: next_mood,
                        intensity: next_intensity,
                        scores: MoodScores::from_single(next_mood),
                        narrative_role: winner::infer_narrative_role(next_mood, next_intensity),
                        finalized: true,
                    })
                })
                .collect::<Vec<_>>();
            let decision = committer.update_with_context(
                Some(chapter_key.as_str()),
                SoundtrackObservation {
                    page: Some(page_number_from_rel_path(&page.rel_path)),
                    mood,
                    intensity,
                    scores: MoodScores::from_single(mood),
                    narrative_role: winner::infer_narrative_role(mood, intensity),
                    finalized: true,
                },
                &future_context,
            );
            committed_map.insert(
                page.rel_path.clone(),
                format!(
                    "{}:{}",
                    decision.committed_mood.as_str(),
                    decision.committed_intensity.as_u8()
                ),
            );
        }
    }

    committed_map
}

fn write_soundtrack_commit_companion(
    results_dir: &Path,
    spec_id: &str,
    chapters: &BTreeMap<String, Vec<PageEntry>>,
    result_map: &HashMap<String, String>,
) -> PathBuf {
    let companion_path =
        results_dir.join(format!("realtest_suite_{}_soundtrack_commit.json", spec_id));
    let committed_map = build_soundtrack_commit_result_map(chapters, result_map);
    let _ = std::fs::write(
        &companion_path,
        serde_json::to_string_pretty(&committed_map).unwrap(),
    );
    companion_path
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
    partial_result_map: &mut HashMap<String, String>,
    partial_cache_path: &Path,
    verbose: bool,
) -> (Vec<WindowPrediction>, u32) {
    let mut results = Vec::new();
    let mut errors = 0u32;

    for center in 0..pages.len() {
        let strict_prod_timing = strict_prod_timing_enabled();
        let total_start = std::time::Instant::now();
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
        let elapsed = if strict_prod_timing {
            total_start.elapsed().as_secs_f64()
        } else {
            start.elapsed().as_secs_f64()
        };
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
            mood: mood.clone(),
            intensity,
            elapsed_s: elapsed,
        });

        // Persist center-page predictions immediately so Ctrl-C still leaves visible progress.
        partial_result_map.insert(
            pages[center].rel_path.clone(),
            format!("{}:{}", mood, intensity),
        );
        persist_result_map_cache(partial_cache_path, partial_result_map);
    }

    (results, errors)
}

fn persist_result_map_cache(cache_path: &Path, result_map: &HashMap<String, String>) {
    let _ = std::fs::write(
        cache_path,
        serde_json::to_string_pretty(result_map).unwrap(),
    );
}

fn persist_result_map_snapshot(
    results_dir: &Path,
    spec_id: &str,
    chapters: &BTreeMap<String, Vec<PageEntry>>,
    cache_path: &Path,
    result_map: &HashMap<String, String>,
    write_soundtrack: bool,
) {
    persist_result_map_cache(cache_path, result_map);
    if write_soundtrack {
        let _ = write_soundtrack_commit_companion(results_dir, spec_id, chapters, result_map);
    }
}

fn build_window_body(
    live: &LiveExperiment,
    prompt: &str,
    images: &[EncodedImage],
    grammar_enabled: bool,
) -> serde_json::Value {
    let winner_images = images
        .iter()
        .map(|image| winner::EncodedImage {
            mime: image.mime,
            b64: image.b64.clone(),
        })
        .collect::<Vec<_>>();
    winner::build_request_body(
        prompt,
        &winner_images,
        live.sampling.max_tokens,
        live.sampling.temperature,
        live.sampling.top_p,
        live.sampling.top_k,
        live.sampling.seed,
        grammar_enabled.then_some(MOOD_GRAMMAR),
    )
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
    winner::build_focus_prompt(page_idx as u32, total_pages as u32)
}

fn build_wide_sequence_prompt(context_indices: &[usize], total_pages: usize) -> String {
    let as_u32 = context_indices
        .iter()
        .map(|idx| *idx as u32)
        .collect::<Vec<_>>();
    winner::build_wide_sequence_prompt(&as_u32, total_pages as u32)
}

fn build_narrative_focus_prompt(page_idx: usize, total_pages: usize) -> String {
    winner::build_narrative_focus_prompt(page_idx as u32, total_pages as u32)
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
    let allow_cache = !strict_prod_timing_enabled();
    let cache_key = format!("{}::{}::{:?}", page.rel_path, prep.as_str(), role);
    if allow_cache {
        if let Some(existing) = cache.get(&cache_key) {
            return existing.clone();
        }
    }

    let winner_prep = match prep {
        ImagePrepMode::Legacy672Jpeg => winner::PrepMode::Legacy672Jpeg,
        ImagePrepMode::CenterPng1024 => winner::PrepMode::CenterPng1024,
        ImagePrepMode::AllPng1024 => winner::PrepMode::AllPng1024,
    };
    let winner_role = match role {
        ImageRole::Context => winner::ImageRole::Context,
        ImageRole::Center => winner::ImageRole::Center,
    };
    let winner_encoded = winner::encode_image(&page.raw_bytes, winner_prep, winner_role)
        .unwrap_or_else(|e| panic!("Failed to prepare {}: {}", page.rel_path, e));
    let encoded = EncodedImage {
        mime: winner_encoded.mime,
        b64: winner_encoded.b64,
    };

    if allow_cache {
        cache.insert(cache_key, encoded.clone());
    }
    encoded
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
    winner::parse_prediction(json)
        .ok()
        .map(|(mood, intensity)| (mood.as_str().to_string(), intensity.as_u8()))
}

fn to_winner_window_predictions(
    window_results: &[WindowPrediction],
) -> Vec<winner::WindowPrediction> {
    window_results
        .iter()
        .filter_map(|window| {
            let mood = BaseMood::from_str_opt(&window.mood)?;
            Some(winner::WindowPrediction {
                members: window.members.iter().map(|idx| *idx as u32).collect(),
                center: window.center as u32,
                mood,
                intensity: MoodIntensity::from_u8(window.intensity.max(1)),
            })
        })
        .collect()
}

fn to_winner_prediction(prediction: &PagePrediction) -> Option<winner::AggregatedPrediction> {
    let mood = BaseMood::from_str_opt(&prediction.mood)?;
    let intensity = MoodIntensity::from_u8(prediction.intensity.max(1));
    Some(winner::AggregatedPrediction {
        mood,
        intensity,
        winner_score: prediction.winner_score,
        runner_up_score: prediction.runner_up_score,
        scores: MoodScores::from_single(mood),
        narrative_role: winner::infer_narrative_role(mood, intensity),
        note: prediction.note.clone(),
    })
}

fn from_winner_prediction(prediction: &winner::AggregatedPrediction) -> PagePrediction {
    PagePrediction {
        mood: prediction.mood.as_str().to_string(),
        intensity: prediction.intensity.as_u8(),
        winner_score: prediction.winner_score,
        runner_up_score: prediction.runner_up_score,
        note: prediction.note.clone(),
    }
}

fn aggregate_majority(pages: &[PageEntry], stats: &[PageVoteStats]) -> Vec<PagePrediction> {
    pages
        .iter()
        .enumerate()
        .map(|(idx, _)| from_winner_prediction(&winner::aggregate_prediction(&stats[idx])))
        .collect()
}

fn aggregate_center_override(pages: &[PageEntry], stats: &[PageVoteStats]) -> Vec<PagePrediction> {
    pages
        .iter()
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
                    let center_intensity = page_stats
                        .center_intensity
                        .map(|intensity| intensity.as_u8())
                        .unwrap_or(2);
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

            let intensity = if final_idx == page_stats.center_vote.map(|m| m.index()).unwrap_or(99)
            {
                page_stats
                    .center_intensity
                    .map(|intensity| intensity.as_u8())
                    .unwrap_or_else(|| {
                        winner::average_intensity(page_stats, final_idx)
                            .map(|intensity| intensity.as_u8())
                            .unwrap_or(2)
                    })
            } else {
                winner::average_intensity(page_stats, final_idx)
                    .map(|intensity| intensity.as_u8())
                    .unwrap_or(2)
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
            let intensity = winner::average_intensity(&stats[idx], *state)
                .map(|value| value.as_u8())
                .unwrap_or(2);
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

async fn apply_wide5_selective_repairs(
    http: &reqwest::Client,
    server_url: &str,
    pages: &[PageEntry],
    mut predictions: Vec<PagePrediction>,
    live: &LiveExperiment,
    cache: &mut HashMap<String, EncodedImage>,
    apply_action_hold: bool,
) -> (Vec<PagePrediction>, u32, f64) {
    let page_order = (0..pages.len()).map(|idx| idx as u32).collect::<Vec<_>>();
    let base_predictions = page_order
        .iter()
        .zip(predictions.iter())
        .filter_map(|(&page, prediction)| Some((page, to_winner_prediction(prediction)?)))
        .collect::<BTreeMap<_, _>>();
    let mut reprompt_cache: HashMap<(usize, SelectivePrompt), Option<(PagePrediction, f64)>> =
        HashMap::new();
    let mut touched_pages = HashSet::new();
    let mut extra_time = 0.0f64;

    for plan in winner::plan_selective_repairs(&page_order, &base_predictions) {
        match plan {
            winner::RepairPlan::Single {
                page,
                prompt_kind,
                reason,
                accept,
                ..
            } => {
                let idx = page as usize;
                if let Some(prediction) = get_selective_prediction(
                    http,
                    server_url,
                    pages,
                    idx,
                    match prompt_kind {
                        winner::SelectivePrompt::Focus => SelectivePrompt::Focus,
                        winner::SelectivePrompt::Narrative => SelectivePrompt::Narrative,
                    },
                    live,
                    cache,
                    &mut reprompt_cache,
                    &mut touched_pages,
                    &mut extra_time,
                )
                .await
                {
                    let should_apply = match accept {
                        winner::RepairAcceptSingle::Tension => prediction.mood == "tension",
                        winner::RepairAcceptSingle::Mystery => prediction.mood == "mystery",
                        winner::RepairAcceptSingle::MysteryOrTension => {
                            matches!(prediction.mood.as_str(), "mystery" | "tension")
                        }
                    };
                    if should_apply {
                        apply_selective_override(&mut predictions[idx], prediction, reason);
                    }
                }
            }
            winner::RepairPlan::Pair {
                first_page,
                second_page,
                prompt_kind,
                reason,
                accept,
                ..
            } => {
                let first_idx = first_page as usize;
                let second_idx = second_page as usize;
                let mapped_prompt = match prompt_kind {
                    winner::SelectivePrompt::Focus => SelectivePrompt::Focus,
                    winner::SelectivePrompt::Narrative => SelectivePrompt::Narrative,
                };
                let first = get_selective_prediction(
                    http,
                    server_url,
                    pages,
                    first_idx,
                    mapped_prompt,
                    live,
                    cache,
                    &mut reprompt_cache,
                    &mut touched_pages,
                    &mut extra_time,
                )
                .await;
                let second = get_selective_prediction(
                    http,
                    server_url,
                    pages,
                    second_idx,
                    mapped_prompt,
                    live,
                    cache,
                    &mut reprompt_cache,
                    &mut touched_pages,
                    &mut extra_time,
                )
                .await;
                let should_apply = match (&first, &second, accept) {
                    (Some(first), Some(second), winner::RepairAcceptPair::BothMystery) => {
                        first.mood == "mystery" && second.mood == "mystery"
                    }
                    (Some(first), Some(second), winner::RepairAcceptPair::BothEpic) => {
                        first.mood == "epic" && second.mood == "epic"
                    }
                    (Some(first), Some(second), winner::RepairAcceptPair::BothTension) => {
                        first.mood == "tension" && second.mood == "tension"
                    }
                    _ => false,
                };
                if should_apply {
                    if let Some(prediction) = first {
                        apply_selective_override(&mut predictions[first_idx], prediction, reason);
                    }
                    if let Some(prediction) = second {
                        apply_selective_override(&mut predictions[second_idx], prediction, reason);
                    }
                }
            }
        }
    }

    if apply_action_hold {
        apply_action_bridge_holds(&page_order, &base_predictions, &mut predictions);
    }

    (predictions, touched_pages.len() as u32, extra_time)
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

fn apply_action_bridge_holds(
    page_order: &[u32],
    base_predictions: &BTreeMap<u32, winner::AggregatedPrediction>,
    predictions: &mut [PagePrediction],
) {
    let mut effective_predictions = page_order
        .iter()
        .filter_map(|&page| {
            predictions
                .get(page as usize)
                .and_then(to_winner_prediction)
                .map(|prediction| (page, prediction))
        })
        .collect::<BTreeMap<_, _>>();

    winner::apply_action_bridge_holds(page_order, base_predictions, &mut effective_predictions);

    for &page in page_order {
        let Some(replacement) = effective_predictions.get(&page) else {
            continue;
        };
        if let Some(target) = predictions.get_mut(page as usize) {
            *target = from_winner_prediction(replacement);
        }
    }
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
    let allow_cache = !strict_prod_timing_enabled();
    if allow_cache {
        if let Some(cached) = reprompt_cache.get(&key) {
            return cached.as_ref().map(|(prediction, _)| prediction.clone());
        }
    }

    let result =
        selective_reprompt_page(http, server_url, pages, idx, prompt_kind, live, cache).await;
    if allow_cache {
        reprompt_cache.insert(key, result.clone());
    }

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
    let strict_prod_timing = strict_prod_timing_enabled();
    let total_start = std::time::Instant::now();
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
    let elapsed = if strict_prod_timing {
        total_start.elapsed().as_secs_f64()
    } else {
        start.elapsed().as_secs_f64()
    };
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
            quantization: "llm=Q4_K_M + mmproj=F16".to_string(),
            prep: "legacy_672_jpeg".to_string(),
            decision: "majority".to_string(),
            sampling: "greedy_hist".to_string(),
            runtime: "historical".to_string(),
            runtime_context_size: None,
            runtime_parallel_slots: None,
            runtime_gpu_layers: None,
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
            weights_mib: None,
            cache_path: None,
            notes: format!("Missing cache: {}", cache_path.display()),
            per_mood: BTreeMap::new(),
            page_results: Vec::new(),
        };
    };
    let _ = write_soundtrack_commit_companion(
        cache_path.parent().unwrap_or_else(|| Path::new(".")),
        spec.id,
        chapters,
        &result_map,
    );

    build_summary_from_results(
        spec,
        "cache",
        "Qwen3-VL-4B-Thinking",
        "llm=Q4_K_M + mmproj=F16",
        "legacy_672_jpeg",
        "majority",
        "greedy_hist",
        "historical",
        0,
        0,
        &[],
        None,
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
    quantization: &str,
    prep: &str,
    decision: &str,
    sampling: &str,
    runtime: &str,
    errors: u32,
    second_pass_pages: u32,
    window_durations: &[f64],
    vram_mib: Option<u32>,
    weights_mib: Option<u64>,
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
    let mut page_results = Vec::new();

    for pages in chapters.values() {
        for page in pages {
            if page.confidence <= 0.0 {
                continue;
            }
            total += 1;
            let entry = per_mood
                .entry(page.mood.clone())
                .or_insert_with(MoodBreakdown::default);
            entry.total += 1;
            if let Some(tag) = result_map.get(&page.rel_path) {
                let mut parts = tag.split(':');
                let got_mood = parts.next().unwrap_or("???");
                let got_intensity = parts.next().and_then(|s| s.parse::<u8>().ok()).unwrap_or(0);
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
                page_results.push(PageResult {
                    rel_path: page.rel_path.clone(),
                    chapter: chapter_label(&page.rel_path),
                    page_number: page_number_from_rel_path(&page.rel_path),
                    expected_mood: page.mood.clone(),
                    expected_intensity: page.intensity,
                    predicted_mood: got_mood.to_string(),
                    predicted_intensity: got_intensity,
                    verdict: verdict_label(strict_match, relaxed_match),
                    intensity_match: got_intensity == page.intensity,
                });
            } else {
                page_results.push(PageResult {
                    rel_path: page.rel_path.clone(),
                    chapter: chapter_label(&page.rel_path),
                    page_number: page_number_from_rel_path(&page.rel_path),
                    expected_mood: page.mood.clone(),
                    expected_intensity: page.intensity,
                    predicted_mood: "???".to_string(),
                    predicted_intensity: 0,
                    verdict: "missing".to_string(),
                    intensity_match: false,
                });
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
        quantization: quantization.to_string(),
        prep: prep.to_string(),
        decision: decision.to_string(),
        sampling: sampling.to_string(),
        runtime: runtime.to_string(),
        runtime_context_size: None,
        runtime_parallel_slots: None,
        runtime_gpu_layers: None,
        strict,
        relaxed,
        intensity,
        total,
        errors,
        second_pass_pages,
        avg_window_s,
        min_window_s: if min_window_s.is_finite() {
            min_window_s
        } else {
            0.0
        },
        max_window_s: if max_window_s.is_finite() {
            max_window_s
        } else {
            0.0
        },
        vram_mib,
        weights_mib,
        cache_path: Some(cache_path.display().to_string()),
        notes: notes.to_string(),
        per_mood,
        page_results,
    }
}

fn verdict_label(strict_match: bool, relaxed_match: bool) -> String {
    if strict_match {
        "strict".to_string()
    } else if relaxed_match {
        "relaxed".to_string()
    } else {
        "wrong".to_string()
    }
}

fn chapter_label(rel_path: &str) -> String {
    let parts = rel_path.split('/').collect::<Vec<_>>();
    if parts.len() >= 2 {
        format!("{}/{}", parts[0], parts[1])
    } else {
        rel_path.to_string()
    }
}

fn page_number_from_rel_path(rel_path: &str) -> u32 {
    rel_path
        .split('/')
        .last()
        .and_then(|name| name.split('.').next())
        .and_then(|stem| stem.parse::<u32>().ok())
        .unwrap_or(0)
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
                "    {DIM}{} — runtime {}, strict {}/{} ({:.1}%), relaxed {}/{} ({:.1}%), intensity {}/{}, avg {:.1}s, VRAM {}, second-pass {}, errors {}{RESET}",
                summary.id,
                summary.runtime,
                summary.strict,
                summary.total,
                strict_pct,
                summary.relaxed,
                summary.total,
                relaxed_pct,
                summary.intensity,
                summary.total,
                summary.avg_window_s,
                summary
                    .vram_mib
                    .map(|value| format!("{value} MiB"))
                    .unwrap_or_else(|| "n/a".to_string()),
                summary.second_pass_pages,
                summary.errors
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
        "  {DIM}| {:<18} | {:<16} | {:<18} | {:<24} | {:<12} | {:>7} | {:>7} | {:>7} | {:>7} | {:>7} | {:>6} | {:>6} | {:>8} |{RESET}",
        "experiment",
        "model",
        "quant",
        "runtime",
        "decision",
        "strict",
        "d_str",
        "relax",
        "d_rel",
        "int",
        "errors",
        "avg_s",
        "vram"
    );

    for summary in ordered {
        let delta_strict = summary.strict as i32 - baseline.strict as i32;
        let delta_relaxed = summary.relaxed as i32 - baseline.relaxed as i32;
        let strict_color = if delta_strict > 0 {
            GREEN
        } else if delta_strict < 0 {
            RED
        } else {
            CYAN
        };
        let relax_color = if delta_relaxed > 0 {
            GREEN
        } else if delta_relaxed < 0 {
            RED
        } else {
            CYAN
        };
        println!(
            "  | {:<18} | {:<16} | {:<18} | {:<24} | {:<12} | {:>3}/{:<3} | {strict_color}{:>+4}{RESET}   | {:>3}/{:<3} | {relax_color}{:>+4}{RESET}   | {:>3}/{:<3} | {:>6} | {:>6.1} | {:>8} |",
            summary.id,
            truncate(&summary.model, 16),
            truncate(&summary.quantization, 18),
            truncate(&summary.runtime, 24),
            truncate(&summary.decision, 12),
            summary.strict,
            summary.total,
            delta_strict,
            summary.relaxed,
            summary.total,
            delta_relaxed,
            summary.intensity,
            summary.total,
            summary.errors,
            summary.avg_window_s,
            summary
                .vram_mib
                .map(|value| value.to_string())
                .unwrap_or_else(|| "n/a".to_string())
        );
    }

    let baseline_label = match baseline.source.as_str() {
        "cache" => "Baseline loaded from cache",
        "live" => "Baseline from live run",
        "derived" => "Baseline from derived run",
        _ => "Baseline",
    };
    println!("\n  {BOLD}{baseline_label}:{RESET} {}", baseline.id);
}

fn print_page_comparison(
    summaries: &[ExperimentSummary],
    chapters: &BTreeMap<String, Vec<PageEntry>>,
) {
    let force_print = env_truthy("REALTEST_PRINT_PAGE_TABLE");
    let ok_summaries = summaries
        .iter()
        .filter(|summary| summary.status == "ok" && !summary.page_results.is_empty())
        .collect::<Vec<_>>();
    if ok_summaries.is_empty() {
        return;
    }
    if ok_summaries.len() > 6 && !force_print {
        println!(
            "\n  {DIM}Per-page comparison not printed to console because {} experiments are active. Detailed files were still written to disk. Set REALTEST_PRINT_PAGE_TABLE=1 to force console output.{RESET}",
            ok_summaries.len()
        );
        return;
    }

    println!("\n  {BOLD}Per-Page Comparison{RESET}");
    for pages in chapters.values() {
        for page in pages {
            if page.confidence <= 0.0 {
                continue;
            }
            let mut line = format!(
                "  {:<18} expected {:<10} {:>1}",
                page.rel_path, page.mood, page.intensity
            );
            for summary in &ok_summaries {
                let cell = summary
                    .page_results
                    .iter()
                    .find(|result| result.rel_path == page.rel_path)
                    .map(|result| {
                        format!(
                            "{} {}:{} {}{}",
                            summary.id,
                            result.predicted_mood,
                            result.predicted_intensity,
                            result.verdict,
                            if result.intensity_match { "" } else { " !int" }
                        )
                    })
                    .unwrap_or_else(|| format!("{} ???:0 missing", summary.id));
                line.push_str(" | ");
                line.push_str(&cell);
            }
            println!("{line}");
        }
    }
}

fn save_suite_summary(
    results_dir: &Path,
    realtest_dir: &Path,
    summaries: &[ExperimentSummary],
    baseline: &ExperimentSummary,
    chapters: &BTreeMap<String, Vec<PageEntry>>,
) {
    let slug = std::env::var("REALTEST_FILTER")
        .ok()
        .map(|f| sanitize_slug(&f))
        .unwrap_or_else(|| "all".to_string());
    let page_suffix = std::env::var("REALTEST_PAGE_LIMIT")
        .ok()
        .map(|v| format!("_first{}", v))
        .unwrap_or_default();
    let selection_suffix = selection_output_slug()
        .map(|value| format!("_{}", value))
        .unwrap_or_default();
    let json_path = results_dir.join(format!(
        "realtest_suite_{}{}{}.json",
        slug, page_suffix, selection_suffix
    ));
    let md_path = results_dir.join(format!(
        "realtest_suite_{}{}{}.md",
        slug, page_suffix, selection_suffix
    ));
    let page_json_path = results_dir.join(format!(
        "realtest_suite_{}{}{}_pages.json",
        slug, page_suffix, selection_suffix
    ));
    let page_md_path = results_dir.join(format!(
        "realtest_suite_{}{}{}_pages.md",
        slug, page_suffix, selection_suffix
    ));

    let payload = serde_json::json!({
        "baseline_id": baseline.id,
        "baseline_strict": baseline.strict,
        "baseline_relaxed": baseline.relaxed,
        "baseline_intensity": baseline.intensity,
        "selection_slug": selection_output_slug(),
        "results_dir": results_dir,
        "realtest_dir": realtest_dir,
        "summaries": summaries,
    });
    let _ = std::fs::write(&json_path, serde_json::to_string_pretty(&payload).unwrap());

    let mut markdown = String::new();
    markdown.push_str("| experiment | model | quantization | runtime | prep | decision | strict | delta_strict | relaxed | delta_relaxed | intensity | errors | avg_window_s | vram_mib | weights_mib |\n");
    markdown.push_str("|---|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|\n");
    for summary in summaries {
        markdown.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {}/{} | {:+} | {}/{} | {:+} | {}/{} | {} | {:.1} | {} | {} |\n",
            summary.id,
            summary.model,
            summary.quantization,
            summary.runtime,
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
            summary.errors,
            summary.avg_window_s,
            summary
                .vram_mib
                .map(|value| value.to_string())
                .unwrap_or_else(|| "n/a".to_string()),
            summary
                .weights_mib
                .map(|value| value.to_string())
                .unwrap_or_else(|| "n/a".to_string())
        ));
    }
    let _ = std::fs::write(&md_path, markdown);

    let ok_summaries = summaries
        .iter()
        .filter(|summary| summary.status == "ok" && !summary.page_results.is_empty())
        .collect::<Vec<_>>();
    let mut page_payload = Vec::new();
    let mut page_markdown = String::new();
    page_markdown.push_str("| page | expected |");
    for summary in &ok_summaries {
        page_markdown.push_str(&format!(" {} | {} verdict |", summary.id, summary.id));
    }
    page_markdown.push('\n');
    page_markdown.push_str("|---|---|");
    for _ in &ok_summaries {
        page_markdown.push_str("---|---|");
    }
    page_markdown.push('\n');

    for pages in chapters.values() {
        for page in pages {
            if page.confidence <= 0.0 {
                continue;
            }

            let mut predictions = serde_json::Map::new();
            page_markdown.push_str(&format!(
                "| {} | {}:{} |",
                page.rel_path, page.mood, page.intensity
            ));

            for summary in &ok_summaries {
                if let Some(result) = summary
                    .page_results
                    .iter()
                    .find(|result| result.rel_path == page.rel_path)
                {
                    predictions.insert(
                        summary.id.clone(),
                        serde_json::json!({
                            "predicted_mood": result.predicted_mood,
                            "predicted_intensity": result.predicted_intensity,
                            "verdict": result.verdict,
                            "intensity_match": result.intensity_match,
                        }),
                    );
                    page_markdown.push_str(&format!(
                        " {}:{} | {}{} |",
                        result.predicted_mood,
                        result.predicted_intensity,
                        result.verdict,
                        if result.intensity_match {
                            ""
                        } else {
                            " (!int)"
                        }
                    ));
                } else {
                    predictions.insert(
                        summary.id.clone(),
                        serde_json::json!({
                            "predicted_mood": "???",
                            "predicted_intensity": 0,
                            "verdict": "missing",
                            "intensity_match": false,
                        }),
                    );
                    page_markdown.push_str(" ???:0 | missing |");
                }
            }
            page_markdown.push('\n');

            page_payload.push(serde_json::json!({
                "page": page.rel_path,
                "chapter": chapter_label(&page.rel_path),
                "page_number": page_number_from_rel_path(&page.rel_path),
                "expected_mood": page.mood,
                "expected_intensity": page.intensity,
                "predictions": predictions,
            }));
        }
    }

    let _ = std::fs::write(
        &page_json_path,
        serde_json::to_string_pretty(&page_payload).unwrap(),
    );
    let _ = std::fs::write(&page_md_path, page_markdown);

    println!(
        "\n  {DIM}Suite summary saved to:{} {} {} {}{RESET}",
        json_path.display(),
        md_path.display(),
        page_json_path.display(),
        page_md_path.display()
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
    value
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
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

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::{apply_action_bridge_holds, to_winner_prediction, PagePrediction};

    fn prediction(mood: &str) -> PagePrediction {
        PagePrediction {
            mood: mood.to_string(),
            intensity: 3,
            winner_score: 0.0,
            runner_up_score: 0.0,
            note: None,
        }
    }

    #[test]
    fn action_bridge_hold_promotes_first_two_tension_pages() {
        let base = [
            "epic", "epic", "tension", "tension", "tension", "tension", "tension", "epic", "epic",
            "epic", "epic",
        ]
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>();
        let mut predictions = base.iter().map(|mood| prediction(mood)).collect::<Vec<_>>();
        let page_order = (0..base.len() as u32).collect::<Vec<_>>();
        let base_predictions = page_order
            .iter()
            .filter_map(|&page| {
                predictions
                    .get(page as usize)
                    .and_then(to_winner_prediction)
                    .map(|prediction| (page, prediction))
            })
            .collect::<BTreeMap<_, _>>();

        apply_action_bridge_holds(&page_order, &base_predictions, &mut predictions);

        assert_eq!(predictions[2].mood, "epic");
        assert_eq!(predictions[3].mood, "epic");
        assert_eq!(predictions[4].mood, "tension");
        assert_eq!(
            predictions[2].note.as_deref(),
            Some("action_bridge_hold_epic")
        );
    }

    #[test]
    fn action_bridge_hold_ignores_long_tension_runs() {
        let base = [
            "epic", "epic", "tension", "tension", "tension", "tension", "tension", "tension",
            "tension", "epic", "epic", "epic", "epic",
        ]
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>();
        let mut predictions = base.iter().map(|mood| prediction(mood)).collect::<Vec<_>>();
        let page_order = (0..base.len() as u32).collect::<Vec<_>>();
        let base_predictions = page_order
            .iter()
            .filter_map(|&page| {
                predictions
                    .get(page as usize)
                    .and_then(to_winner_prediction)
                    .map(|prediction| (page, prediction))
            })
            .collect::<BTreeMap<_, _>>();

        apply_action_bridge_holds(&page_order, &base_predictions, &mut predictions);

        assert_eq!(predictions[2].mood, "tension");
        assert_eq!(predictions[3].mood, "tension");
    }
}
