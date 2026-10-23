//! Live discovery run against real YouTube, for manual checks only:
//! `KTM_PROFILE=path/to/profile.json KTM_YT_DLP=path/to/yt-dlp \
//!  cargo test -p keytomusic-core --test live_discovery -- --ignored --nocapture`

use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Instant;

use keytomusic_core::discovery::engine::{DiscoveryEngine, SeedInfo};

#[tokio::test]
#[ignore = "network: hits YouTube through yt-dlp"]
async fn live_discovery_from_profile() {
    let profile = std::env::var("KTM_PROFILE").expect("KTM_PROFILE");
    let yt_dlp = std::env::var("KTM_YT_DLP").expect("KTM_YT_DLP");
    let json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(profile).unwrap()).unwrap();
    let sounds = json["sounds"].as_array().unwrap();
    let seeds: Vec<SeedInfo> = sounds
        .iter()
        .filter_map(|s| {
            Some(SeedInfo {
                video_id: s["resolvedVideoId"].as_str()?.to_string(),
                sound_name: s["name"].as_str().unwrap_or("").to_string(),
            })
        })
        .collect();
    let existing: Vec<String> = seeds.iter().map(|s| s.video_id.clone()).collect();
    println!("{} seeds", seeds.len());

    let engine = DiscoveryEngine::new(Arc::new(AtomicBool::new(false)));
    let start = Instant::now();
    let suggestions = engine
        .generate_suggestions(
            seeds,
            existing.clone(),
            Vec::new(),
            yt_dlp.into(),
            |done, total, name| {
                println!(
                    "[{:>5.1}s] {done}/{total} {name}",
                    start.elapsed().as_secs_f32()
                )
            },
            |_| {},
        )
        .await;
    println!(
        "done in {:.1}s, {} suggestions",
        start.elapsed().as_secs_f32(),
        suggestions.len()
    );
    for s in suggestions.iter().take(30) {
        println!(
            "{:>2} seeds  {:>4.0}s  {}  — {}",
            s.occurrence_count, s.duration, s.title, s.channel
        );
    }
    assert!(!suggestions.is_empty());
    assert!(suggestions.iter().all(|s| !existing.contains(&s.video_id)));
    let mut ids: Vec<_> = suggestions.iter().map(|s| &s.video_id).collect();
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), suggestions.len(), "duplicate suggestions");
}
