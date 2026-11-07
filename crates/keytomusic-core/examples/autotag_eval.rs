//! Measure the default moods on a profile whose keys are already tagged: the
//! tags are hidden, suggested again, and compared.
//!
//! ```sh
//! cargo run -p keytomusic-core --example autotag_eval -- path/to/profile.json \
//!     [--chatgpt] [--model gpt-6.1-sol] [--no-names] [--no-model]
//! ```
//!
//! `--no-names` also hides the key names (they often say the mood);
//! untagged keys of the profile are listed with what they would get.

use keytomusic_core::mood::autotag;
use keytomusic_core::mood::chatgpt;
use keytomusic_core::mood::provider::{AuthMode, ProviderSettings};
use keytomusic_core::types::{MoodImageDetail, Profile};

fn arg(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1).cloned())
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let path = args.first().expect("profile.json");
    let mut profile: Profile =
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).expect("profile");
    let plan = args.iter().any(|a| a == "--chatgpt");
    if plan && !chatgpt::global().can_use_plan() {
        eprintln!("--chatgpt: sign in with ChatGPT in the app first");
        std::process::exit(2);
    }
    let settings = ProviderSettings {
        auth: if plan {
            AuthMode::ChatGpt { plan: true }
        } else {
            AuthMode::ApiKey
        },
        base_url: arg(&args, "--base-url").unwrap_or_else(|| "https://api.openai.com/v1".into()),
        model: arg(&args, "--model").unwrap_or_else(|| "gpt-6.1-sol".into()),
        api_key: std::env::var("OPENAI_API_KEY").ok(),
        image_detail: MoodImageDetail::default(),
    };
    let use_model = !args.iter().any(|a| a == "--no-model");

    let truth: Vec<_> = profile
        .key_bindings
        .iter()
        .map(|b| {
            (
                b.key_code.clone(),
                b.mood,
                b.mood_intensity.map_or(1, |i| i.as_u8()),
            )
        })
        .collect();
    let untagged: Vec<String> = profile
        .key_bindings
        .iter()
        .filter(|b| b.mood.is_none())
        .map(|b| b.key_code.clone())
        .collect();
    for s in &mut profile.sounds {
        s.mood = None;
        s.mood_intensity = None;
    }
    for b in &mut profile.key_bindings {
        b.mood = None;
        b.mood_intensity = None;
        if args.iter().any(|a| a == "--no-names") {
            b.name = None;
        }
    }

    let started = std::time::Instant::now();
    let result = autotag::suggest(use_model.then_some(&settings), &profile).await;
    println!(
        "{} suggestions, {} unknown, model {} ({:?}) in {:.1}s",
        result.suggestions.len(),
        result.unknown.len(),
        result.used_model,
        result.model_error,
        started.elapsed().as_secs_f32()
    );
    let (mut moods, mut levels, mut tagged, mut recommended_right, mut recommended) =
        (0, 0, 0, 0, 0);
    for s in &result.suggestions {
        let Some((_, mood, level)) = truth.iter().find(|(k, ..)| *k == s.key_code) else {
            continue;
        };
        let mark = match mood {
            Some(m) => {
                tagged += 1;
                let right = *m == s.mood;
                moods += usize::from(right);
                levels += usize::from(right && *level == s.intensity.as_u8());
                if s.recommended() {
                    recommended += 1;
                    recommended_right += usize::from(right);
                }
                if right {
                    "ok "
                } else {
                    "XX "
                }
            }
            None => "new",
        };
        println!(
            "{mark} {:<12} {:<9} {} {:?}/{:?}  (was {:?} {})  {}",
            s.key_code,
            s.mood.as_str(),
            s.intensity.as_u8(),
            s.certainty,
            s.source,
            mood.map(|m| m.as_str()),
            level,
            s.label.chars().take(60).collect::<String>()
        );
    }
    for label in &result.unknown {
        println!("??? {label}");
    }
    println!(
        "mood right {moods}/{tagged}, mood+intensity {levels}/{tagged}, recommended right {recommended_right}/{recommended}; {} keys were untagged",
        untagged.len()
    );
}
