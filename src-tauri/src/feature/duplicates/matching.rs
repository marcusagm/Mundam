//! Deterministic comparison math, outside SQLite transactions and the async executor.

use crate::core::{duplicates::matching::*, models::DuplicateGroupType};
use std::collections::BTreeMap;
use tokio_util::sync::CancellationToken;

#[cfg(test)]
#[path = "matching_tests.rs"]
mod tests;

/// Returns no result on cancellation, rather than publishing a partially evaluated prefix.
pub fn compare(input: &MatchingInput, token: &CancellationToken) -> Option<Vec<MatchingGroup>> {
    if token.is_cancelled() {
        return None;
    }
    match input.ticket.kind {
        MatchingKind::Exact => exact(input, token),
        MatchingKind::Visual => visual(input, token),
    }
}

fn candidate(asset_id: String, score: f64, reasons: Vec<String>) -> MatchingCandidate {
    MatchingCandidate {
        asset_id,
        score,
        reasons,
    }
}

fn exact(input: &MatchingInput, token: &CancellationToken) -> Option<Vec<MatchingGroup>> {
    if !input.policy.consider_exact_match {
        return Some(Vec::new());
    }
    let mut hashes: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    for fingerprint in &input.fingerprints {
        if token.is_cancelled() {
            return None;
        }
        if let Some(hash) = &fingerprint.content_hash {
            hashes
                .entry(hash)
                .or_default()
                .push(fingerprint.asset_id.clone());
        }
    }
    Some(
        hashes
            .into_values()
            .filter(|members| members.len() > 1)
            .map(|members| MatchingGroup {
                group_type: DuplicateGroupType::Exact,
                confidence: 1.0,
                candidates: members
                    .into_iter()
                    .map(|asset_id| candidate(asset_id, 1.0, vec!["exact_content_hash".into()]))
                    .collect(),
            })
            .collect(),
    )
}

fn visual(input: &MatchingInput, token: &CancellationToken) -> Option<Vec<MatchingGroup>> {
    let policy = &input.policy;
    if !policy.consider_visual_match && !policy.consider_crop_match {
        return Some(Vec::new());
    }
    let mut candidates = Vec::with_capacity(input.fingerprints.len());
    for fingerprint in &input.fingerprints {
        if token.is_cancelled() {
            return None;
        }
        if let Some(hash) = fingerprint
            .perceptual_hash
            .as_deref()
            .and_then(|value| u64::from_str_radix(value, 16).ok())
        {
            let variants = super::matcher::generate_hash_variants(
                hash,
                policy.allow_rotation,
                policy.allow_mirroring,
            );
            candidates.push((fingerprint, hash, variants));
        }
    }
    let visual_threshold = if policy.min_visual_score > 0.0 {
        policy.min_visual_score
    } else {
        policy.min_score
    };
    let crop_threshold = if policy.min_crop_score > 0.0 {
        policy.min_crop_score
    } else {
        policy.min_score
    };
    let mut grouped = vec![false; candidates.len()];
    let mut groups = Vec::new();
    for primary_index in 0..candidates.len() {
        if token.is_cancelled() {
            return None;
        }
        if grouped[primary_index] {
            continue;
        }
        let (primary, primary_hash, _) = &candidates[primary_index];
        let mut members = vec![candidate(
            primary.asset_id.clone(),
            1.0,
            vec!["canonical_anchor".into()],
        )];
        let mut group_type = DuplicateGroupType::Visual;
        let mut scores = Vec::new();
        for comparison_index in primary_index + 1..candidates.len() {
            if token.is_cancelled() {
                return None;
            }
            if grouped[comparison_index] {
                continue;
            }
            let (comparison, comparison_hash, comparison_variants) = &candidates[comparison_index];
            if primary.content_hash == comparison.content_hash {
                continue;
            }
            let evaluation = super::matcher::evaluate_visual_and_crop_match_prepared(
                *primary_hash,
                *comparison_hash,
                comparison_variants,
                primary.block_hash.as_deref(),
                comparison.block_hash.as_deref(),
                primary.thumb_hash.as_deref(),
                comparison.thumb_hash.as_deref(),
                (primary.width, primary.height),
                (comparison.width, comparison.height),
                primary.file_size,
                comparison.file_size,
                visual_threshold,
                crop_threshold,
                policy.consider_visual_match,
                policy.consider_crop_match,
                policy.allow_rotation,
                policy.allow_mirroring,
            );
            if let Some(evaluation) = evaluation {
                if evaluation.match_type == DuplicateGroupType::Derived {
                    group_type = DuplicateGroupType::Derived;
                }
                scores.push(evaluation.score);
                members.push(candidate(
                    comparison.asset_id.clone(),
                    evaluation.score,
                    evaluation.reasons,
                ));
                grouped[comparison_index] = true;
            }
        }
        if members.len() > 1 {
            grouped[primary_index] = true;
            groups.push(MatchingGroup {
                group_type,
                confidence: scores.iter().sum::<f64>() / scores.len() as f64,
                candidates: members,
            });
        }
    }
    Some(groups)
}
