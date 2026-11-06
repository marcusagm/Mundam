use super::*;
use crate::core::models::{DuplicateFingerprint, DuplicateRuleSet};

#[test]
fn processing_prepared_matching_preserves_anchor_order_exact_exclusion_and_cancellation(
) -> Result<(), Box<dyn std::error::Error>> {
    let now = chrono::Utc::now();
    let fingerprint = |asset: &str, content: &str, hash| DuplicateFingerprint {
        asset_id: asset.into(),
        content_hash: Some(content.into()),
        perceptual_hash: Some(format!("{hash:016x}")),
        block_hash: None,
        thumb_hash: None,
        width: Some(32),
        height: Some(32),
        file_size: Some(8),
        mime_type: None,
        format_family: None,
        color_profile: None,
        orientation: None,
        fingerprint_version: 5,
        updated_at: now,
    };
    let hash = 0x4777ac2327478753_u64;
    let input = MatchingInput {
        ticket: MatchingTicket {
            kind: MatchingKind::Visual,
            lease_id: "test".into(),
            revision: 1,
            manual_run_id: None,
        },
        policy: DuplicateRuleSet {
            id: "test".into(),
            name: "Test".into(),
            description: None,
            consider_exact_match: true,
            consider_visual_match: true,
            consider_crop_match: true,
            ignore_resolution_difference: true,
            ignore_recompression: true,
            allow_rotation: true,
            allow_mirroring: true,
            min_score: 1.0,
            min_visual_score: 1.0,
            min_crop_score: 1.0,
            created_at: now,
            updated_at: now,
        },
        fingerprints: vec![
            fingerprint("anchor", "one", hash),
            fingerprint(
                "rotated",
                "two",
                super::super::matcher::rotate_dhash_90_clockwise(hash),
            ),
            fingerprint("zero", "zero", 0),
            fingerprint("exact", "one", hash),
        ],
    };
    let token = CancellationToken::new();
    let groups = compare(&input, &token).ok_or("Expected a value in processing_prepared_matching_preserves_anchor_order_exact_exclusion_and_cancellation")?;
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].group_type, DuplicateGroupType::Visual);
    assert_eq!(
        groups[0]
            .candidates
            .iter()
            .map(|candidate| candidate.asset_id.as_str())
            .collect::<Vec<_>>(),
        ["anchor", "rotated"]
    );
    assert_eq!(groups[0].confidence, 1.0);
    token.cancel();
    assert!(compare(&input, &token).is_none());

    Ok(())
}
