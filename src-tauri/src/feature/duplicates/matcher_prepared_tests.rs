use super::*;

#[test]
fn processing_prepared_variants_preserve_scores_reasons_and_optional_evidence() {
    let hashes = [0, 1, u64::MAX, 0x4777ac2327478753, 0x37224bc239472781];
    let blocks = [
        None,
        Some("invalid"),
        Some("1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"),
    ];
    for rotation in [false, true] {
        for mirroring in [false, true] {
            for hash_one in hashes {
                for hash_two in hashes {
                    let variants = generate_hash_variants(hash_two, rotation, mirroring);
                    for block in blocks {
                        for (visual, crop) in
                            [(true, true), (true, false), (false, true), (false, false)]
                        {
                            let expected = evaluate_visual_and_crop_match(
                                hash_one,
                                hash_two,
                                block,
                                block,
                                block,
                                block,
                                (Some(32), Some(64)),
                                (Some(64), Some(32)),
                                Some(10),
                                Some(20),
                                0.72,
                                0.85,
                                visual,
                                crop,
                                rotation,
                                mirroring,
                            );
                            let actual = evaluate_visual_and_crop_match_prepared(
                                hash_one,
                                hash_two,
                                &variants,
                                block,
                                block,
                                block,
                                block,
                                (Some(32), Some(64)),
                                (Some(64), Some(32)),
                                Some(10),
                                Some(20),
                                0.72,
                                0.85,
                                visual,
                                crop,
                                rotation,
                                mirroring,
                            );
                            assert_eq!(
                                actual.as_ref().map(|result| (
                                    &result.match_type,
                                    result.score,
                                    &result.reasons
                                )),
                                expected.as_ref().map(|result| (
                                    &result.match_type,
                                    result.score,
                                    &result.reasons
                                ))
                            );
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn processing_prepared_crop_keeps_multiscale_only_corroboration_and_sparse_rejection(
) -> Result<(), Box<dyn std::error::Error>> {
    let block = "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";
    let variants = generate_hash_variants(0x5555555555555555, false, false);
    let evaluate = |multiscale| {
        evaluate_visual_and_crop_match_prepared(
            0xaaaaaaaaaaaaaaaa,
            0x5555555555555555,
            &variants,
            Some(block),
            Some(block),
            multiscale,
            multiscale,
            (None, None),
            (None, None),
            None,
            None,
            0.9,
            0.9,
            false,
            true,
            false,
            false,
        )
    };
    assert!(evaluate(None).is_none());
    let zero_multiscale = "0".repeat(64);
    assert!(evaluate(Some(&zero_multiscale)).is_none());
    let matched = evaluate(Some(block)).ok_or("Expected a value in processing_prepared_crop_keeps_multiscale_only_corroboration_and_sparse_rejection")?;
    assert_eq!(matched.match_type, DuplicateGroupType::Derived);
    assert_eq!(matched.score, 1.0);
    assert!(matched
        .reasons
        .iter()
        .any(|reason| reason == "multiscale_similarity:100%"));

    Ok(())
}
