//! Standalone optimized benchmark of the production pair evaluator, excluding extraction and SQLite.

#[allow(dead_code)]
mod core {
    pub mod models {
        #[derive(Debug, Clone, PartialEq, Eq, Hash)]
        pub enum DuplicateGroupType {
            Exact,
            Visual,
            Derived,
        }
    }
}
#[allow(dead_code)]
#[path = "../src-tauri/src/feature/duplicates/matcher.rs"]
mod matcher;

use std::{
    hash::{Hash, Hasher},
    hint::black_box,
    time::Instant,
};

struct Sample {
    hash: u64,
    block: String,
    multiscale: String,
}

fn random(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}

fn samples(count: usize, related: bool, corroborated: bool) -> Vec<Sample> {
    let mut state = 0x4777ac2327478753;
    (0..count)
        .map(|index| {
            let hash = if related {
                0x4777ac2327478753 ^ (1 << (index % 64))
            } else {
                random(&mut state)
            };
            let mut sample = Sample {
                hash,
                block: format!(
                    "{:016x}{:016x}{:016x}{:016x}",
                    random(&mut state),
                    random(&mut state),
                    random(&mut state),
                    random(&mut state)
                ),
                multiscale: format!(
                    "{:016x}{:016x}{:016x}{:016x}",
                    random(&mut state),
                    random(&mut state),
                    random(&mut state),
                    random(&mut state)
                ),
            };
            if corroborated {
                let evidence = "1234567890abcdef".repeat(4);
                sample.block = evidence.clone();
                sample.multiscale = evidence;
            }
            sample
        })
        .collect()
}

fn run(samples: &[Sample], crop: bool) -> (u128, u64) {
    let started = Instant::now();
    #[cfg(prepared)]
    let variants: Vec<_> = samples
        .iter()
        .map(|sample| matcher::generate_hash_variants(sample.hash, true, true))
        .collect();
    let mut digest = std::collections::hash_map::DefaultHasher::new();
    for primary in 0..samples.len() {
        for comparison in primary + 1..samples.len() {
            let first = black_box(&samples[primary]);
            let second = black_box(&samples[comparison]);
            #[cfg(not(prepared))]
            let result = matcher::evaluate_visual_and_crop_match(
                first.hash,
                second.hash,
                Some(&first.block),
                Some(&second.block),
                Some(&first.multiscale),
                Some(&second.multiscale),
                (Some(100), Some(100)),
                (Some(200), Some(200)),
                Some(100),
                Some(200),
                0.9,
                0.9,
                true,
                crop,
                true,
                true,
            );
            #[cfg(prepared)]
            let result = matcher::evaluate_visual_and_crop_match_prepared(
                first.hash,
                second.hash,
                &variants[comparison],
                Some(&first.block),
                Some(&second.block),
                Some(&first.multiscale),
                Some(&second.multiscale),
                (Some(100), Some(100)),
                (Some(200), Some(200)),
                Some(100),
                Some(200),
                0.9,
                0.9,
                true,
                crop,
                true,
                true,
            );
            if let Some(result) = result {
                primary.hash(&mut digest);
                comparison.hash(&mut digest);
                result.match_type.hash(&mut digest);
                result.score.to_bits().hash(&mut digest);
                result.reasons.hash(&mut digest);
            }
        }
    }
    (started.elapsed().as_micros(), digest.finish())
}

fn main() {
    println!("# Pair evaluator; rustc -O; arch={}; os={}; three samples, median; deterministic seed; preparation included", std::env::consts::ARCH, std::env::consts::OS);
    println!("scenario\tassets\tpairs\tmedian_us\tdigest");
    for (scenario, related, crop, corroborated) in [
        ("unrelated_crop", false, true, false),
        ("unrelated_visual", false, false, false),
        ("related_crop", true, true, false),
        ("crop_corroborated", false, true, true),
    ] {
        for count in [128, 512, 1024] {
            let samples = samples(count, related, corroborated);
            let mut timings = Vec::new();
            let mut expected = None;
            for _ in 0..3 {
                let (elapsed, digest) = run(&samples, crop);
                assert!(expected.is_none_or(|value| value == digest));
                expected = Some(digest);
                timings.push(elapsed);
            }
            timings.sort_unstable();
            println!(
                "{scenario}\t{count}\t{}\t{}\t{}",
                count * (count - 1) / 2,
                timings[1],
                expected.unwrap()
            );
        }
    }
}
