//! Analyze a chapter from its reader URL, without the browser: download the
//! page list like the extension does, let the Mood AI engine download and
//! analyze the pages, print the scenes.
//!
//! ```sh
//! cargo run --release -p keytomusic-core --example mood_chapter -- \
//!     https://sushiscan.net/one-piece-chapitre-1180/ [--model gpt-6.1-sol] [--save dir]
//! ```
//! Uses the ChatGPT plan of the account signed in in the app.

use std::sync::Arc;
use std::time::{Duration, Instant};

use keytomusic_core::events::{CoreEvent, EventSink};
use keytomusic_core::mood::chatgpt;
use keytomusic_core::mood::engine::MoodEngine;
use keytomusic_core::mood::fetch;
use keytomusic_core::mood::provider::{AuthMode, ProviderSettings};
use keytomusic_core::types::MoodImageDetail;

struct Printer(Instant);
impl EventSink for Printer {
    fn emit(&self, event: CoreEvent) {
        if let CoreEvent::MoodCommitted {
            mood,
            intensity,
            summary,
            ..
        } = &event
        {
            println!(
                "[{:>5.1}s] PLAY {mood} {intensity} — {summary}",
                self.0.elapsed().as_secs_f32()
            );
        }
    }
}

fn arg(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1).cloned())
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let url = args.first().expect("chapter URL").clone();
    if !chatgpt::global().can_use_plan() {
        eprintln!("Sign in with ChatGPT in the app first");
        std::process::exit(2);
    }
    let client = fetch::client();
    let html = client.get(&url).send().await.unwrap().text().await.unwrap();
    let images = fetch::ts_reader_images(&html).expect("no ts_reader page list");
    println!("{} pages", images.len());
    let engine = Arc::new(MoodEngine::new(None));
    engine.configure(ProviderSettings {
        auth: AuthMode::ChatGpt { plan: true },
        base_url: "https://api.openai.com/v1".into(),
        model: arg(&args, "--model").unwrap_or_else(|| "gpt-6.1-sol".into()),
        api_key: None,
        image_detail: MoodImageDetail::High,
    });
    let started = Instant::now();
    engine.start(Arc::new(Printer(started)));
    // Like the extension: the page list, the app downloads.
    engine.open_chapter(&url, Some("chapter".into()), Some(images.len() as u32));
    engine.set_sources(&url, images.clone(), Some(url.clone()));
    engine.set_position(&url, 0, None);
    let mut downloaded_at = None;
    loop {
        tokio::time::sleep(Duration::from_millis(500)).await;
        let snapshot = engine.snapshot().unwrap();
        if downloaded_at.is_none() && snapshot.received >= snapshot.total_pages {
            downloaded_at = Some(started.elapsed());
            println!(
                "downloaded {} pages in {:.1}s (wanted from the extension: {:?})",
                snapshot.received,
                started.elapsed().as_secs_f32(),
                snapshot.wanted
            );
        }
        if snapshot.analyzed >= snapshot.total_pages || started.elapsed() > Duration::from_secs(600)
        {
            println!(
                "analyzed {}/{} in {:.1}s, error {:?}",
                snapshot.analyzed,
                snapshot.total_pages,
                started.elapsed().as_secs_f32(),
                snapshot.error
            );
            for (page, mood) in &snapshot.pages {
                println!(
                    "  page {:>2}: {:<9} {} {}",
                    page + 1,
                    mood.mood.as_str(),
                    mood.intensity,
                    mood.summary
                );
            }
            break;
        }
    }
}
