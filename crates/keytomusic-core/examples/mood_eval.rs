//! Measure Mood AI on an annotated dataset (one folder per chapter, a
//! `ground_truth.json` mapping "SERIES/CHAPTER/page.jpg" to `{mood, intensity}`).
//!
//! ```sh
//! OPENAI_API_KEY=… cargo run -p keytomusic-core --example mood_eval -- \
//!     path/to/RealTest [--chapters BL/2,OP/1] [--model gpt-5-mini] [--detail high] \
//!     [--base-url https://api.openai.com/v1] [--out results.json]
//! ```
//!
//! Every chapter goes through the same batching and prompt as the app.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use base64::Engine as _;
use keytomusic_core::mood::analyzer::{self, PageLabel};
use keytomusic_core::mood::chapter::{Chapter, Next};
use keytomusic_core::mood::provider::ProviderSettings;
use keytomusic_core::mood::server::prepare_image;
use keytomusic_core::types::{BaseMood, MoodImageDetail};
use serde_json::{json, Value};

fn family(mood: BaseMood) -> u8 {
    match mood {
        BaseMood::Epic | BaseMood::Tension => 0,
        BaseMood::Sadness | BaseMood::Romance => 1,
        BaseMood::Comedy | BaseMood::Peaceful => 2,
        BaseMood::Horror | BaseMood::Mystery => 3,
    }
}

fn arg(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1).cloned())
}

async fn run_chapter(
    settings: &ProviderSettings,
    dir: &Path,
    files: &[String],
) -> Result<BTreeMap<u32, PageLabel>, String> {
    let mut chapter = Chapter::new(dir.display().to_string());
    chapter.total_pages = Some(files.len() as u32);
    let long_ago = Instant::now() - std::time::Duration::from_secs(10);
    for (index, file) in files.iter().enumerate() {
        let bytes = std::fs::read(dir.join(file)).map_err(|e| e.to_string())?;
        let b64 = base64::engine::general_purpose::STANDARD.encode(bytes);
        let image = prepare_image(&b64)?;
        chapter.add_page(index as u32, Arc::new(image), long_ago);
    }
    loop {
        match chapter.next_batch(Instant::now()) {
            Next::Run(plan) => {
                chapter.start(&plan);
                let mut attempt = 0;
                loop {
                    attempt += 1;
                    match analyzer::analyze(settings, &plan.pages, &plan.context).await {
                        Ok(labels) => {
                            chapter.finish(&plan, labels);
                            break;
                        }
                        Err(e) if e.retryable && attempt < 4 => {
                            eprintln!("  retry ({}): {}", attempt, e.message);
                            tokio::time::sleep(std::time::Duration::from_secs(2 * attempt)).await;
                        }
                        Err(e) => return Err(e.message),
                    }
                }
            }
            Next::Wait(delay) => tokio::time::sleep(delay).await,
            Next::Idle => break,
        }
    }
    Ok(chapter.labels)
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(root) = args
        .first()
        .filter(|a| !a.starts_with("--"))
        .map(PathBuf::from)
    else {
        eprintln!("usage: mood_eval <dataset dir> [--chapters A/1,B/2] [--model m] [--detail high|low] [--base-url url] [--out file]");
        std::process::exit(2);
    };
    let base_url = arg(&args, "--base-url").unwrap_or_else(|| "https://api.openai.com/v1".into());
    let settings = ProviderSettings {
        api_key: std::env::var("KEYTOMUSIC_MOOD_API_KEY")
            .or_else(|_| std::env::var("OPENAI_API_KEY"))
            .ok(),
        model: arg(&args, "--model").unwrap_or_else(|| "gpt-5-mini".into()),
        image_detail: arg(&args, "--detail")
            .and_then(|d| MoodImageDetail::from_str_opt(&d))
            .unwrap_or_default(),
        base_url,
    };
    let truth: HashMap<String, Value> = serde_json::from_slice(
        &std::fs::read(root.join("ground_truth.json")).expect("ground_truth.json"),
    )
    .expect("valid ground truth");

    let mut chapters: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for key in truth.keys() {
        if let Some((chapter, file)) = key.rsplit_once('/') {
            chapters
                .entry(chapter.to_string())
                .or_default()
                .push(file.to_string());
        }
    }
    if let Some(wanted) = arg(&args, "--chapters") {
        let wanted: Vec<&str> = wanted.split(',').collect();
        chapters.retain(|chapter, _| {
            wanted
                .iter()
                .any(|w| chapter == w || chapter.starts_with(&format!("{w}/")))
        });
    }
    for files in chapters.values_mut() {
        files.sort();
    }

    let started = Instant::now();
    let mut rows = Vec::new();
    let (mut total, mut exact, mut fam, mut tension) = (0usize, 0usize, 0usize, 0usize);
    let (mut predicted_changes, mut true_changes) = (0usize, 0usize);
    for (chapter, files) in &chapters {
        let t = Instant::now();
        let labels = match run_chapter(&settings, &root.join(chapter), files).await {
            Ok(labels) => labels,
            Err(error) => {
                eprintln!("{chapter}: FAILED {error}");
                continue;
            }
        };
        let (mut c_exact, mut c_total) = (0, 0);
        let mut previous: Option<(BaseMood, BaseMood)> = None;
        for (index, file) in files.iter().enumerate() {
            let key = format!("{chapter}/{file}");
            let Some(expected) = truth[&key]["mood"]
                .as_str()
                .and_then(BaseMood::from_str_opt)
            else {
                continue;
            };
            let Some(label) = labels.get(&(index as u32)) else {
                continue;
            };
            total += 1;
            c_total += 1;
            if label.mood == expected {
                exact += 1;
                c_exact += 1;
            }
            if family(label.mood) == family(expected) {
                fam += 1;
            }
            if expected == BaseMood::Tension {
                tension += 1;
            }
            if let Some((p, e)) = previous {
                predicted_changes += usize::from(p != label.mood);
                true_changes += usize::from(e != expected);
            }
            previous = Some((label.mood, expected));
            rows.push(json!({
                "page": key,
                "expected": expected.as_str(),
                "predicted": label.mood.as_str(),
                "intensity": label.intensity.as_u8(),
                "summary": label.summary,
            }));
        }
        println!(
            "{chapter:>8}: {c_exact:>3}/{c_total:<3} exact  ({:.0}s)",
            t.elapsed().as_secs_f32()
        );
    }
    let pct = |n: usize| 100.0 * n as f64 / total.max(1) as f64;
    println!(
        "\n{} · {} pages · exact {:.0}% · family {:.0}% · always-tension baseline {:.0}% · mood changes {} (reference {}) · {:.0}s",
        settings.model,
        total,
        pct(exact),
        pct(fam),
        pct(tension),
        predicted_changes,
        true_changes,
        started.elapsed().as_secs_f32()
    );
    if let Some(out) = arg(&args, "--out") {
        std::fs::write(&out, serde_json::to_vec_pretty(&rows).unwrap()).expect("write results");
        println!("details: {out}");
    }
}
