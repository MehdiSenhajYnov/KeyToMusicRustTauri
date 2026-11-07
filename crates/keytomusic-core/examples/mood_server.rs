//! Run the Mood AI API alone (no audio, no UI) to try the browser extension:
//! prints what the app would play as the reader moves.
//!
//! ```sh
//! cargo run --release -p keytomusic-core --example mood_server -- [--port 8765] [--model gpt-6.1-sol]
//! ```
//! Uses the ChatGPT plan of the account signed in in the app.

use std::sync::Arc;
use std::time::{Duration, Instant};

use keytomusic_core::events::{CoreEvent, EventSink};
use keytomusic_core::mood::chatgpt;
use keytomusic_core::mood::engine::MoodEngine;
use keytomusic_core::mood::provider::{AuthMode, ProviderSettings};
use keytomusic_core::mood::server;
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
                "[{:>6.1}s] PLAY {mood} {intensity} — {summary}",
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
    tracing_subscriber::fmt()
        .with_env_filter("keytomusic_core=info")
        .init();
    let args: Vec<String> = std::env::args().skip(1).collect();
    let port: u16 = arg(&args, "--port")
        .and_then(|p| p.parse().ok())
        .unwrap_or(8765);
    if !chatgpt::global().can_use_plan() {
        eprintln!("Sign in with ChatGPT in the app first");
        std::process::exit(2);
    }
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
    let _server = server::start(port, engine.clone()).await.expect("port");
    println!("listening on 127.0.0.1:{port}");
    let mut last = String::new();
    loop {
        tokio::time::sleep(Duration::from_millis(400)).await;
        if let Some(s) = engine.snapshot() {
            let line = format!(
                "{} | page {}/{} | received {} downloading {} wanted {:?} | analyzed {} in flight {} | now {:?} | error {:?}",
                s.key, s.focus + 1, s.total_pages, s.received, s.downloading, s.wanted, s.analyzed, s.in_flight,
                s.current.as_ref().map(|c| format!("{} {}", c.mood.as_str(), c.intensity)), s.error
            );
            if line != last {
                println!("[{:>6.1}s] {line}", started.elapsed().as_secs_f32());
                last = line;
            }
        }
    }
}
