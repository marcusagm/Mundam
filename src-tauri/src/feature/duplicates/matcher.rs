use crate::core::models::DuplicateGroupType;

/// The evaluated relationship result between two candidate assets.
#[derive(Debug, Clone)]
pub struct DuplicateMatchResult {
    /// The resulting group type (Near/Visual or Derived).
    pub match_type: DuplicateGroupType,
    /// The calculated confidence/similarity score (0.0 to 1.0).
    pub score: f64,
    /// Explainable human-readable and machine-parseable reason tags.
    pub reasons: Vec<String>,
}

// ────────────────────────────────────────────────────────────────────────────
// dHash (8×8 = 64-bit) bit-permutation routines
// ────────────────────────────────────────────────────────────────────────────
//
// A 64-bit dHash encodes an 8×8 gradient grid. Bit index `row * 8 + col`
// maps to position (row, col) in the grid. Rotating or flipping the grid
// requires remapping every bit to its new position.

/// Rotates a 64-bit dHash 90° clockwise.
///
/// Maps `(row, col) → (col, 7 - row)`.
pub fn rotate_dhash_90_clockwise(hash: u64) -> u64 {
    let mut rotated: u64 = 0;
    for row in 0..8u32 {
        for col in 0..8u32 {
            let source_bit_index = row * 8 + col;
            let destination_bit_index = col * 8 + (7 - row);
            if hash & (1u64 << source_bit_index) != 0 {
                rotated |= 1u64 << destination_bit_index;
            }
        }
    }
    rotated
}

/// Rotates a 64-bit dHash 180°.
/// Rotates a 64-bit dHash 180°.
///
/// Maps `(row, col) → (7 - row, 7 - col)` and inverts the gradient direction
/// because left ↔ right is reversed (`pixel_left > pixel_right` becomes `pixel_right > pixel_left`).
pub fn rotate_dhash_180(hash: u64) -> u64 {
    let mut rotated: u64 = 0;
    for row in 0..8u32 {
        for col in 0..8u32 {
            let source_bit_index = row * 8 + col;
            let destination_bit_index = (7 - row) * 8 + (7 - col);
            if hash & (1u64 << source_bit_index) == 0 {
                rotated |= 1u64 << destination_bit_index;
            }
        }
    }
    rotated
}

/// Rotates a 64-bit dHash 270° clockwise (= 90° counter-clockwise).
///
/// Maps `(row, col) → (7 - col, row)`.
pub fn rotate_dhash_270_clockwise(hash: u64) -> u64 {
    let mut rotated: u64 = 0;
    for row in 0..8u32 {
        for col in 0..8u32 {
            let source_bit_index = row * 8 + col;
            let destination_bit_index = (7 - col) * 8 + row;
            if hash & (1u64 << source_bit_index) != 0 {
                rotated |= 1u64 << destination_bit_index;
            }
        }
    }
    rotated
}

/// Mirrors a 64-bit dHash horizontally (left ↔ right flip).
///
/// Reverses column order within each 8-bit row and inverts the gradient bit:
/// `(row, col) → (row, 7 - col)` with `bit = !bit`.
pub fn mirror_dhash_horizontal(hash: u64) -> u64 {
    let mut mirrored: u64 = 0;
    for row in 0..8u32 {
        for col in 0..8u32 {
            let source_bit_index = row * 8 + col;
            let destination_bit_index = row * 8 + (7 - col);
            if hash & (1u64 << source_bit_index) == 0 {
                mirrored |= 1u64 << destination_bit_index;
            }
        }
    }
    mirrored
}

/// Mirrors a 64-bit dHash vertically (top ↔ bottom flip).
///
/// Reverses row order: `(row, col) → (7 - row, col)`. Does not invert the
/// gradient bit because horizontal direction is preserved.
pub fn mirror_dhash_vertical(hash: u64) -> u64 {
    let mut mirrored: u64 = 0;
    for row in 0..8u32 {
        for col in 0..8u32 {
            let source_bit_index = row * 8 + col;
            let destination_bit_index = (7 - row) * 8 + col;
            if hash & (1u64 << source_bit_index) != 0 {
                mirrored |= 1u64 << destination_bit_index;
            }
        }
    }
    mirrored
}

/// Generates all relevant hash variants for comparison based on
/// rotation and mirroring flags.
///
/// Returns a `Vec` containing the original hash plus all enabled
/// permutations. The caller should compare each variant and take
/// the maximum similarity score.
pub fn generate_hash_variants(
    hash: u64,
    allow_rotation: bool,
    allow_mirroring: bool,
) -> Vec<u64> {
    let mut variants: Vec<u64> = Vec::with_capacity(8);
    variants.push(hash);

    if allow_rotation {
        variants.push(rotate_dhash_90_clockwise(hash));
        variants.push(rotate_dhash_180(hash));
        variants.push(rotate_dhash_270_clockwise(hash));
    }

    if allow_mirroring {
        variants.push(mirror_dhash_horizontal(hash));
        variants.push(mirror_dhash_vertical(hash));
        if allow_rotation {
            variants.push(rotate_dhash_90_clockwise(mirror_dhash_horizontal(hash)));
            variants.push(rotate_dhash_270_clockwise(mirror_dhash_horizontal(hash)));
        }
    }

    variants
}

// ────────────────────────────────────────────────────────────────────────────
// Similarity scoring functions
// ────────────────────────────────────────────────────────────────────────────

/// Calculates visual similarity between two 64-bit gradient difference hashes.
///
/// Computes normalized Hamming distance: `1.0 - (hamming_distance / 64.0)`.
///
/// # Arguments
/// * `hash_one` - The 64-bit dHash of the first image.
/// * `hash_two` - The 64-bit dHash of the second image.
///
/// # Returns
/// A float between 0.0 (completely dissimilar) and 1.0 (identical perceptual gradient).
pub fn calculate_visual_similarity(hash_one: u64, hash_two: u64) -> f64 {
    if hash_one == 0 || hash_two == 0 {
        return 0.0;
    }

    let hamming_distance = (hash_one ^ hash_two).count_ones();
    let hamming_similarity = 1.0 - (hamming_distance as f64 / 64.0);

    let shared_ones = (hash_one & hash_two).count_ones();
    let total_ones = (hash_one | hash_two).count_ones();
    if total_ones == 0 {
        return 0.0;
    }

    let ones_one = hash_one.count_ones();
    let ones_two = hash_two.count_ones();
    let minimum_ones = ones_one.min(ones_two);

    // If either hash is sparse (< 22 active bits out of 64), the Hamming distance
    // is artificially inflated by mutual zero bits (representing flat backgrounds/borders).
    // We constrain the similarity by the active feature overlap (Jaccard similarity).
    if minimum_ones < 22 {
        let jaccard_similarity = shared_ones as f64 / total_ones as f64;
        let feature_constrained_similarity = jaccard_similarity * 1.5;
        hamming_similarity.min(feature_constrained_similarity)
    } else {
        hamming_similarity
    }
}

/// Calculates the best visual similarity between `hash_one` and all variants
/// of `hash_two`, considering rotation and mirroring when enabled.
///
/// When `allow_rotation` or `allow_mirroring` is true, generates permuted
/// variants of `hash_two` and returns the maximum similarity found.
pub fn calculate_visual_similarity_with_variants(
    hash_one: u64,
    hash_two: u64,
    allow_rotation: bool,
    allow_mirroring: bool,
) -> f64 {
    if !allow_rotation && !allow_mirroring {
        return calculate_visual_similarity(hash_one, hash_two);
    }

    let hash_two_variants = generate_hash_variants(hash_two, allow_rotation, allow_mirroring);
    let mut best_similarity: f64 = 0.0;

    for variant_hash in hash_two_variants {
        let current_similarity = calculate_visual_similarity(hash_one, variant_hash);
        if current_similarity > best_similarity {
            best_similarity = current_similarity;
        }
    }

    best_similarity
}

/// Calculates multi-scale similarity between two 256-bit hexadecimal hash strings.
///
/// Incorporates feature overlap validation to prevent sparse/transparent images from falsely
/// matching due to mutual background zero bits.
///
/// # Arguments
/// * `multi_scale_hex_one` - The 64-character hex string of the first image.
/// * `multi_scale_hex_two` - The 64-character hex string of the second image.
///
/// # Returns
/// A float between 0.0 and 1.0.
pub fn calculate_multiscale_similarity(
    multi_scale_hex_one: &str,
    multi_scale_hex_two: &str,
) -> f64 {
    if multi_scale_hex_one.len() < 64 || multi_scale_hex_two.len() < 64 {
        return 0.0;
    }

    let mut total_distance: u32 = 0;
    let mut total_shared_ones: u32 = 0;
    let mut total_ones: u32 = 0;
    let mut total_ones_one: u32 = 0;
    let mut total_ones_two: u32 = 0;

    for chunk_index in 0..4 {
        let start_index = chunk_index * 16;
        let end_index = (chunk_index + 1) * 16;
        let slice_one = &multi_scale_hex_one[start_index..end_index];
        let slice_two = &multi_scale_hex_two[start_index..end_index];

        let value_one = u64::from_str_radix(slice_one, 16).unwrap_or(0);
        let value_two = u64::from_str_radix(slice_two, 16).unwrap_or(0);

        total_distance += (value_one ^ value_two).count_ones();
        total_shared_ones += (value_one & value_two).count_ones();
        total_ones += (value_one | value_two).count_ones();
        total_ones_one += value_one.count_ones();
        total_ones_two += value_two.count_ones();
    }

    let hamming_similarity = 1.0 - (total_distance as f64 / 256.0);
    if total_ones == 0 {
        return 0.0;
    }

    let minimum_ones = total_ones_one.min(total_ones_two);
    // In 256 bits, if either hash has fewer than 80 ones (< 31% active bits),
    // it is dominated by uniform/transparent regions. Constrain by active features.
    if minimum_ones < 80 {
        let jaccard_similarity = total_shared_ones as f64 / total_ones as f64;
        let feature_constrained_similarity = jaccard_similarity * 1.5;
        hamming_similarity.min(feature_constrained_similarity)
    } else {
        hamming_similarity
    }
}

/// Calculates both 2x2 and 3x3 sub-grid crop similarities between two spatial block hashes.
///
/// Returns a tuple `(score_two_by_two, score_three_by_three)`.
pub fn calculate_crop_subgrid_scores(block_hash_one: &str, block_hash_two: &str) -> (f64, f64) {
    if block_hash_one.len() < 64 || block_hash_two.len() < 64 {
        return (0.0, 0.0);
    }

    let mut parsed_tiles_one: Vec<u16> = Vec::with_capacity(16);
    let mut parsed_tiles_two: Vec<u16> = Vec::with_capacity(16);

    for tile_index in 0..16 {
        let start_index = tile_index * 4;
        let end_index = (tile_index + 1) * 4;
        let tile_slice_one = &block_hash_one[start_index..end_index];
        let tile_slice_two = &block_hash_two[start_index..end_index];

        parsed_tiles_one.push(u16::from_str_radix(tile_slice_one, 16).unwrap_or(0));
        parsed_tiles_two.push(u16::from_str_radix(tile_slice_two, 16).unwrap_or(0));
    }

    let score_two_by_two_forward =
        evaluate_subgrid_overlap(&parsed_tiles_one, &parsed_tiles_two, 2);
    let score_two_by_two_backward =
        evaluate_subgrid_overlap(&parsed_tiles_two, &parsed_tiles_one, 2);
    let score_three_by_three_forward =
        evaluate_subgrid_overlap(&parsed_tiles_one, &parsed_tiles_two, 3);
    let score_three_by_three_backward =
        evaluate_subgrid_overlap(&parsed_tiles_two, &parsed_tiles_one, 3);

    (
        score_two_by_two_forward.max(score_two_by_two_backward),
        score_three_by_three_forward.max(score_three_by_three_backward),
    )
}

/// Calculates crop similarity between two spatial block hashes (16 tiles x 16 bits = 64 hex characters).
///
/// Tests sub-grid overlaps (2x2 and 3x3 tiles) across spatial positions in both directions to detect
/// whether one image is a cropped or framed sub-region of the other.
///
/// # Arguments
/// * `block_hash_one` - The 64-character hex block hash string of the first image.
/// * `block_hash_two` - The 64-character hex block hash string of the second image.
///
/// # Returns
/// The maximum sub-grid similarity score between 0.0 and 1.0.
pub fn calculate_crop_similarity(block_hash_one: &str, block_hash_two: &str) -> f64 {
    let (score_two_by_two, score_three_by_three) =
        calculate_crop_subgrid_scores(block_hash_one, block_hash_two);
    score_two_by_two.max(score_three_by_three)
}

/// Helper function to evaluate subgrid overlap similarity between a centered source grid
/// and all possible target window positions in a 4x4 grid.
fn evaluate_subgrid_overlap(
    source_tiles: &[u16],
    target_tiles: &[u16],
    sub_grid_dimension: usize,
) -> f64 {
    let offset_start = (4 - sub_grid_dimension) / 2;
    let mut source_subgrid: Vec<u16> = Vec::with_capacity(sub_grid_dimension * sub_grid_dimension);

    for source_row in offset_start..(offset_start + sub_grid_dimension) {
        for source_col in offset_start..(offset_start + sub_grid_dimension) {
            source_subgrid.push(source_tiles[source_row * 4 + source_col]);
        }
    }

    let total_subgrid_bits = (sub_grid_dimension * sub_grid_dimension * 16) as u32;
    let mut minimum_bit_distance = total_subgrid_bits;

    let maximum_shift = 4 - sub_grid_dimension;
    for target_row_shift in 0..=maximum_shift {
        for target_col_shift in 0..=maximum_shift {
            let mut current_window_distance: u32 = 0;
            let mut element_index: usize = 0;

            for window_row in target_row_shift..(target_row_shift + sub_grid_dimension) {
                for window_col in target_col_shift..(target_col_shift + sub_grid_dimension) {
                    let source_tile_value = source_subgrid[element_index];
                    let target_tile_value = target_tiles[window_row * 4 + window_col];
                    current_window_distance +=
                        (source_tile_value ^ target_tile_value).count_ones();
                    element_index += 1;
                }
            }

            if current_window_distance < minimum_bit_distance {
                minimum_bit_distance = current_window_distance;
            }
        }
    }

    1.0 - (minimum_bit_distance as f64 / total_subgrid_bits as f64)
}

/// Evaluates whether two image assets should be matched under the active duplicate rule set,
/// considering perceptual similarity, multi-scale texture, and spatial crop coverage.
///
/// # Arguments
/// * `hash_one` - 64-bit dHash of the first image.
/// * `hash_two` - 64-bit dHash of the second image.
/// * `block_hash_one` - Spatial block hash of the first image.
/// * `block_hash_two` - Spatial block hash of the second image.
/// * `multi_scale_one` - Multi-scale hash of the first image.
/// * `multi_scale_two` - Multi-scale hash of the second image.
/// * `dimension_one` - Dimensions (width, height) of the first image.
/// * `dimension_two` - Dimensions (width, height) of the second image.
/// * `file_size_one` - File size in bytes of the first image.
/// * `file_size_two` - File size in bytes of the second image.
/// * `visual_score_threshold` - Minimum confidence score for visual/perceptual matches.
/// * `crop_score_threshold` - Minimum confidence score for crop/derived matches.
/// * `consider_visual_match` - Whether visual/perceptual matching is enabled in the active rule set.
/// * `consider_crop_match` - Whether crop matching is enabled in the active rule set.
/// * `allow_rotation` - Whether to test rotated (90°/180°/270°) hash variants.
/// * `allow_mirroring` - Whether to test horizontally mirrored hash variants.
///
/// # Returns
/// An `Option<DuplicateMatchResult>` if the similarity exceeds the minimum threshold.
#[allow(clippy::too_many_arguments)]
pub fn evaluate_visual_and_crop_match(
    hash_one: u64,
    hash_two: u64,
    block_hash_one: Option<&str>,
    block_hash_two: Option<&str>,
    multi_scale_one: Option<&str>,
    multi_scale_two: Option<&str>,
    dimension_one: (Option<i32>, Option<i32>),
    dimension_two: (Option<i32>, Option<i32>),
    file_size_one: Option<i64>,
    file_size_two: Option<i64>,
    visual_score_threshold: f64,
    crop_score_threshold: f64,
    consider_visual_match: bool,
    consider_crop_match: bool,
    allow_rotation: bool,
    allow_mirroring: bool,
) -> Option<DuplicateMatchResult> {
    // Guard: Reject degenerate zero-hash images. Images that decode as fully uniform
    // (black, white, or failed decode) produce hash 0x0, which has zero visual entropy
    // and would falsely match other images.
    if hash_one == 0 || hash_two == 0 {
        return None;
    }

    let visual_similarity = calculate_visual_similarity_with_variants(
        hash_one,
        hash_two,
        allow_rotation,
        allow_mirroring,
    );

    let actual_multiscale_similarity = match (multi_scale_one, multi_scale_two) {
        (Some(first_hex), Some(second_hex)) => {
            Some(calculate_multiscale_similarity(first_hex, second_hex))
        }
        _ => None,
    };

    // Primary check: Direct visual/perceptual match
    if consider_visual_match && visual_similarity >= visual_score_threshold {
        // When multiscale hash is available and visual similarity is below 0.85,
        // require multiscale similarity >= 0.65 to ensure the match is not a low-entropy
        // artifact of matching background zeros.
        let multiscale_is_reliable = actual_multiscale_similarity.map_or(true, |similarity_value| {
            visual_similarity >= 0.85 || similarity_value >= 0.65
        });

        if multiscale_is_reliable {
            let mut explainable_reasons: Vec<String> = Vec::new();
            let percentage_formatted = (visual_similarity * 100.0).round() as u32;
            explainable_reasons.push(format!("visual_similarity:{}%", percentage_formatted));

            if allow_rotation || allow_mirroring {
                explainable_reasons.push("rotation_mirroring_checked".to_string());
            }

            if let (Some(width_one), Some(height_one), Some(width_two), Some(height_two)) = (
                dimension_one.0,
                dimension_one.1,
                dimension_two.0,
                dimension_two.1,
            ) {
                if width_one != width_two || height_one != height_two {
                    explainable_reasons.push(format!(
                        "resolution_variation:{}x{}_vs_{}x{}",
                        width_one, height_one, width_two, height_two
                    ));
                }
            }

            if let (Some(size_one), Some(size_two)) = (file_size_one, file_size_two) {
                if size_one != size_two {
                    explainable_reasons.push("recompression_detected".to_string());
                }
            }

            return Some(DuplicateMatchResult {
                match_type: DuplicateGroupType::Visual,
                score: visual_similarity,
                reasons: explainable_reasons,
            });
        }
    }

    // Secondary check: Crop or partial derivative match
    // The 2x2 sub-grid window has only 64 bits of entropy across 18 window positions, which
    // produces an accidental noise floor of ~72% overlap between completely unrelated images.
    // To prevent false positives, we require an independent corroborating signal: either visual
    // similarity or actual multiscale similarity must reach the corroboration floor (>= 0.72)
    // to confirm the images share genuine visual structure beyond random chance and flat-border noise.
    const CROP_CORROBORATION_FLOOR: f64 = 0.72;

    if consider_crop_match {
        let (crop_two_by_two, crop_three_by_three) = match (block_hash_one, block_hash_two) {
            (Some(first_block), Some(second_block)) => {
                let is_zero_block_one = first_block.chars().all(|character| character == '0');
                let is_zero_block_two = second_block.chars().all(|character| character == '0');
                if is_zero_block_one || is_zero_block_two {
                    (0.0, 0.0)
                } else {
                    calculate_crop_subgrid_scores(first_block, second_block)
                }
            }
            _ => (0.0, 0.0),
        };

        let has_corroborating_signal = visual_similarity >= CROP_CORROBORATION_FLOOR
            || actual_multiscale_similarity.is_some_and(|similarity_value| {
                similarity_value >= CROP_CORROBORATION_FLOOR
            });

        if has_corroborating_signal {
            // 3x3 subgrid covers 56.25% of the image (144 bits of entropy)
            // 2x2 subgrid covers only 25% of the image (64 bits of entropy), so it requires
            // a stricter threshold (>= 0.85) to overcome the multi-position noise floor.
            let matches_three_by_three = crop_three_by_three >= crop_score_threshold;
            let matches_two_by_two =
                crop_two_by_two >= 0.85 && crop_two_by_two >= crop_score_threshold;

            if matches_three_by_three || matches_two_by_two {
                let accepted_score = if matches_three_by_three {
                    crop_three_by_three.max(crop_two_by_two)
                } else {
                    crop_two_by_two
                };

                let mut explainable_reasons: Vec<String> = Vec::new();
                let crop_percentage = (accepted_score * 100.0).round() as u32;
                explainable_reasons.push(format!("crop_detected:{}%", crop_percentage));

                if let Some(actual_multiscale) = actual_multiscale_similarity {
                    if actual_multiscale >= CROP_CORROBORATION_FLOOR {
                        let multiscale_percentage = (actual_multiscale * 100.0).round() as u32;
                        explainable_reasons.push(format!(
                            "multiscale_similarity:{}%",
                            multiscale_percentage
                        ));
                    }
                }

                if let (Some(width_one), Some(height_one), Some(width_two), Some(height_two)) = (
                    dimension_one.0,
                    dimension_one.1,
                    dimension_two.0,
                    dimension_two.1,
                ) {
                    if width_one != width_two || height_one != height_two {
                        explainable_reasons.push(format!(
                            "resolution_variation:{}x{}_vs_{}x{}",
                            width_one, height_one, width_two, height_two
                        ));
                    }
                }

                return Some(DuplicateMatchResult {
                    match_type: DuplicateGroupType::Derived,
                    score: accepted_score,
                    reasons: explainable_reasons,
                });
            }
        }
    }

    None
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identical_hashes_yield_one_hundred_percent_similarity() {
        let sample_hash: u64 = 0x4777ac2327478753;
        let calculated_similarity = calculate_visual_similarity(sample_hash, sample_hash);
        assert!((calculated_similarity - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_casa_one_and_casa_two_similarity() {
        let casa_one_hash: u64 = 0x4777ac2327478753;
        let casa_two_hash: u64 = 0x4777a42323478753;
        let calculated_similarity = calculate_visual_similarity(casa_one_hash, casa_two_hash);
        assert!(calculated_similarity >= 0.95);
    }

    #[test]
    fn test_crop_evaluation_matches_under_crop_rule() {
        let dummy_block_hash = "828867f76c673667089566c82211fff97157ccd81144effd6722c8085107fffd";
        let crop_score = calculate_crop_similarity(dummy_block_hash, dummy_block_hash);
        assert!((crop_score - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_zero_hash_pair_is_rejected_as_degenerate() {
        let result = evaluate_visual_and_crop_match(
            0,
            0,
            None,
            None,
            None,
            None,
            (Some(100), Some(100)),
            (Some(100), Some(100)),
            Some(1000),
            Some(2000),
            0.70,
            0.70,
            true,
            false,
            false,
            false,
        );
        assert!(
            result.is_none(),
            "Two zero hashes should never match — they are degenerate"
        );
    }

    #[test]
    fn test_single_zero_hash_is_rejected() {
        let valid_hash: u64 = 0x4777ac2327478753;
        let result = evaluate_visual_and_crop_match(
            0,
            valid_hash,
            None,
            None,
            None,
            None,
            (Some(100), Some(100)),
            (Some(100), Some(100)),
            Some(1000),
            Some(2000),
            0.70,
            0.70,
            true,
            true,
            false,
            false,
        );
        assert!(
            result.is_none(),
            "A zero hash compared to a valid hash should be rejected"
        );
    }

    #[test]
    fn test_crop_match_without_corroboration_is_rejected() {
        // olka vs casa2: visual ~47%, multiscale None, crop ~72%
        // Crop exceeds 70% threshold but visual is below corroboration floor (0.72)
        let olka_hash: u64 = 0xb1957762a39ba986;
        let casa_hash: u64 = 0x4777ac2327478753;

        let result = evaluate_visual_and_crop_match(
            olka_hash,
            casa_hash,
            Some("a6ac6cd90a6a26554a555488a221deb5d114ccd8585dabe976b3aaaa956edeaf"),
            Some("00bc411cfb87fbbdccccc2e87c045ccccccc40004575cccccdc814195311ccc7"),
            None,
            None,
            (Some(1440), Some(809)),
            (Some(1000), Some(566)),
            Some(500000),
            Some(300000),
            0.70,
            0.70,
            true,
            true,
            false,
            false,
        );
        assert!(
            result.is_none(),
            "Crop match without corroborating visual/multiscale signal should be rejected"
        );
    }

    #[test]
    fn test_olka_three_and_hasselblad_ppm_are_rejected() {
        // Real false positive reported by user: olka 3.png vs RAW_HASSELBLAD_CFV.PPM
        // Visual: 57.8%, Multiscale: 47.3%, Crop: 70.3%
        let olka_three_hash: u64 = 0x179521e2a3618bab;
        let hasselblad_hash: u64 = 0x9ed759bf38716ae2;

        let result = evaluate_visual_and_crop_match(
            olka_three_hash,
            hasselblad_hash,
            Some("2a22ba56633591131b952a8a55446e9cdd39eecc4a6a25acc74784aaaeec7732"),
            Some("6c9ea42bb9551225564bb1adc4b3cec0767755452ad6215ba9ac646c9ad6a1aa"),
            Some("0a7513799571ce53921bdb92a90d3884a48f8eaf28aeadafc9e7668feaa3b2a2"),
            Some("5bf1e5d296d8679d13531ace4cdcbb326626776825527f613696d5a95550ae6e"),
            (Some(1292), Some(646)),
            (Some(4096), Some(4096)),
            Some(1000000),
            Some(50000000),
            0.70,
            0.70,
            true,
            true,
            false,
            false,
        );
        assert!(
            result.is_none(),
            "olka 3 and RAW_HASSELBLAD_CFV.PPM false positive crop match must be rejected"
        );
    }

    #[test]
    fn test_olka_three_and_casa_two_are_rejected() {
        // Real false positive: olka 3.png vs a casa 2.png
        // Visual: 60.9%, Multiscale: 51.2%, 2x2 Crop: 71.9%, 3x3 Crop: 60.4%
        let olka_three_hash: u64 = 0x179521e2a3618bab;
        let casa_two_hash: u64 = 0x4777ac2327478753;

        let result = evaluate_visual_and_crop_match(
            olka_three_hash,
            casa_two_hash,
            Some("2a22ba56633591131b952a8a55446e9cdd39eecc4a6a25acc74784aaaeec7732"),
            Some("00bc411cfb87fbbdccccc2e87c045ccccccc40004575cccccdc814195311ccc7"),
            Some("0a7513799571ce53921bdb92a90d3884a48f8eaf28aeadafc9e7668feaa3b2a2"),
            Some("017f31a7a18ba77dab7dac76a786a95ea57ec17ea17eb15ea17ec55ea0deab1f"),
            (Some(1292), Some(646)),
            (Some(1000), Some(566)),
            Some(1000000),
            Some(300000),
            0.70,
            0.70,
            true,
            true,
            false,
            false,
        );
        assert!(
            result.is_none(),
            "olka 3 and a casa 2 false positive crop match must be rejected"
        );
    }

    #[test]
    fn test_low_entropy_e797_and_10e7_are_rejected() {
        // Real false positive: e797 vs 10e7 (flat-background mutual zeros match 70% in 64-bit dHash, but 50.8% in multiscale)
        let hash_one: u64 = 0x02208cf6a484c404;
        let hash_two: u64 = 0x14800cb616a402a6;

        let result = evaluate_visual_and_crop_match(
            hash_one,
            hash_two,
            Some("0000000000000000000200190184005076563aa5244000002a2091405a400880"),
            Some("412c8900a00311000ffe1ffe0777448a605420401e08a40080a8008026462004"),
            Some("000000000000000000080008119329807100a2a0aac05090894881a015001000"),
            Some("92268d00000080020192f77ef77ed2da215e00e8248810220022005288460022"),
            (Some(1024), Some(768)),
            (Some(1024), Some(768)),
            Some(40000),
            Some(36000),
            0.70,
            0.70,
            true,
            true,
            false,
            false,
        );
        assert!(
            result.is_none(),
            "Low-entropy flat background collision without multiscale corroboration must be rejected"
        );
    }

    #[test]
    fn test_single_afphoto_and_original_4646_are_rejected() {
        // Real false positive: single.afphoto_thumb.png vs original-4646e3522ed6ed4c7aad938a2e866524.webp
        let thumb_hash: u64 = 0x0501014905010101;
        let webp_hash: u64 = 0x06e4da9518527ac0;

        let result = evaluate_visual_and_crop_match(
            thumb_hash,
            webp_hash,
            Some("480100000000161160000000000011310200000000003311000400000000111d"),
            Some("0001000c00000311efb266418880191344451999752553330610004000513777"),
            Some("0000010100810081400176617a694001641565b5400100030043008000800000"),
            Some("0000000300019801e629e609e429008140f5485148cbd8d10003000520410011"),
            (Some(252), Some(512)),
            (Some(752), Some(564)),
            Some(30000),
            Some(29000),
            0.70,
            0.70,
            true,
            true,
            false,
            false,
        );
        assert!(
            result.is_none(),
            "Transparent background thumbnail false positive must be rejected"
        );
    }

    #[test]
    fn test_genuine_visual_match_is_accepted() {
        // casa1 vs casa2: identical hashes → 100% visual match
        let casa_hash: u64 = 0x4777ac2327478753;

        let result = evaluate_visual_and_crop_match(
            casa_hash,
            casa_hash,
            None,
            None,
            None,
            None,
            (Some(1000), Some(566)),
            (Some(1000), Some(566)),
            Some(500000),
            Some(500000),
            0.70,
            0.70,
            true,
            false,
            false,
            false,
        );
        assert!(result.is_some(), "Identical hashes should always match");
        assert!((result.unwrap().score - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_visual_match_skipped_when_disabled_even_if_identical() {
        let casa_hash: u64 = 0x4777ac2327478753;

        let result = evaluate_visual_and_crop_match(
            casa_hash,
            casa_hash,
            None,
            None,
            None,
            None,
            (Some(1000), Some(566)),
            (Some(1000), Some(566)),
            Some(500000),
            Some(500000),
            0.70,
            0.70,
            false,
            false,
            false,
            false,
        );
        assert!(
            result.is_none(),
            "When consider_visual_match is false and consider_crop_match is false, no visual match should form"
        );
    }

    // ────────────────────────────────────────────────────────────────────────
    // Rotation / mirroring bit-permutation tests
    // ────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_rotate_90_four_times_returns_original() {
        let original_hash: u64 = 0x4777ac2327478753;
        let once_rotated = rotate_dhash_90_clockwise(original_hash);
        let twice_rotated = rotate_dhash_90_clockwise(once_rotated);
        let thrice_rotated = rotate_dhash_90_clockwise(twice_rotated);
        let fully_rotated = rotate_dhash_90_clockwise(thrice_rotated);
        assert_eq!(
            original_hash, fully_rotated,
            "Four 90° rotations must return to the original hash"
        );
    }

    #[test]
    fn test_rotate_180_is_involution() {
        let original_hash: u64 = 0x4777ac2327478753;
        let double_rotated = rotate_dhash_180(rotate_dhash_180(original_hash));
        assert_eq!(
            original_hash, double_rotated,
            "Two 180° rotations must return to the original hash"
        );
    }

    #[test]
    fn test_rotate_270_is_inverse_of_rotate_90() {
        let original_hash: u64 = 0x4777ac2327478753;
        let rotated_90_then_270 = rotate_dhash_270_clockwise(rotate_dhash_90_clockwise(original_hash));
        assert_eq!(
            original_hash, rotated_90_then_270,
            "90° CW followed by 270° CW must return to the original hash"
        );
    }

    #[test]
    fn test_rotate_90_followed_by_270_returns_original() {
        let original_hash: u64 = 0xb1957762a39ba986;
        let rotated_270_then_90 = rotate_dhash_90_clockwise(rotate_dhash_270_clockwise(original_hash));
        assert_eq!(
            original_hash, rotated_270_then_90,
            "270° CW followed by 90° CW must return to the original hash"
        );
    }

    #[test]
    fn test_mirror_horizontal_is_involution() {
        let original_hash: u64 = 0x4777ac2327478753;
        let double_mirrored = mirror_dhash_horizontal(mirror_dhash_horizontal(original_hash));
        assert_eq!(
            original_hash, double_mirrored,
            "Two horizontal mirrors must return to the original hash"
        );
    }

    #[test]
    fn test_mirror_vertical_is_involution() {
        let original_hash: u64 = 0x4777ac2327478753;
        let double_mirrored = mirror_dhash_vertical(mirror_dhash_vertical(original_hash));
        assert_eq!(
            original_hash, double_mirrored,
            "Two vertical mirrors must return to the original hash"
        );
    }

    #[test]
    fn test_rotation_changes_the_hash() {
        let original_hash: u64 = 0x4777ac2327478753;
        let rotated_hash = rotate_dhash_90_clockwise(original_hash);
        assert_ne!(
            original_hash, rotated_hash,
            "A non-symmetric hash should change after 90° rotation"
        );
    }

    #[test]
    fn test_mirror_changes_the_hash() {
        let original_hash: u64 = 0x4777ac2327478753;
        let mirrored_hash = mirror_dhash_horizontal(original_hash);
        assert_ne!(
            original_hash, mirrored_hash,
            "A non-symmetric hash should change after horizontal mirror"
        );
    }

    #[test]
    fn test_generate_variants_rotation_only() {
        let original_hash: u64 = 0x4777ac2327478753;
        let variants = generate_hash_variants(original_hash, true, false);
        assert_eq!(variants.len(), 4, "Rotation-only should produce 4 variants (0°, 90°, 180°, 270°)");
        assert_eq!(variants[0], original_hash, "First variant should be the original hash");
    }

    #[test]
    fn test_generate_variants_mirror_only() {
        let original_hash: u64 = 0x4777ac2327478753;
        let variants = generate_hash_variants(original_hash, false, true);
        assert_eq!(variants.len(), 3, "Mirror-only should produce 3 variants (original + horizontal + vertical)");
    }

    #[test]
    fn test_generate_variants_rotation_and_mirror() {
        let original_hash: u64 = 0x4777ac2327478753;
        let variants = generate_hash_variants(original_hash, true, true);
        assert_eq!(variants.len(), 8, "Rotation + mirror should produce 8 variants (4 rotations × 2 mirror states)");
    }

    #[test]
    fn test_generate_variants_neither_produces_single_entry() {
        let original_hash: u64 = 0x4777ac2327478753;
        let variants = generate_hash_variants(original_hash, false, false);
        assert_eq!(variants.len(), 1, "No rotation or mirror should produce 1 variant (original only)");
    }

    #[test]
    fn test_rotated_hash_matches_original_with_rotation_enabled() {
        let original_hash: u64 = 0x4777ac2327478753;
        let rotated_hash = rotate_dhash_90_clockwise(original_hash);

        // Without rotation: similarity should be significantly less than 1.0
        let without_rotation = calculate_visual_similarity(original_hash, rotated_hash);
        assert!(
            without_rotation < 0.95,
            "Rotated hash should not be near-identical to original without rotation handling"
        );

        // With rotation: should find perfect match via variant search
        let with_rotation = calculate_visual_similarity_with_variants(
            original_hash,
            rotated_hash,
            true,
            false,
        );
        assert!(
            (with_rotation - 1.0).abs() < f64::EPSILON,
            "Rotated hash should match 100% when rotation is enabled"
        );
    }

    #[test]
    fn test_mirrored_hash_matches_original_with_mirroring_enabled() {
        let original_hash: u64 = 0x4777ac2327478753;
        let mirrored_hash = mirror_dhash_horizontal(original_hash);

        // Without mirroring: similarity should be less than perfect
        let without_mirroring = calculate_visual_similarity(original_hash, mirrored_hash);
        assert!(
            without_mirroring < 0.95,
            "Mirrored hash should not be near-identical to original without mirror handling"
        );

        // With mirroring: should find perfect match
        let with_mirroring = calculate_visual_similarity_with_variants(
            original_hash,
            mirrored_hash,
            false,
            true,
        );
        assert!(
            (with_mirroring - 1.0).abs() < f64::EPSILON,
            "Mirrored hash should match 100% when mirroring is enabled"
        );
    }

    #[test]
    fn test_rotated_image_matched_via_evaluate_with_rotation() {
        let original_hash: u64 = 0x4777ac2327478753;
        let rotated_hash = rotate_dhash_180(original_hash);

        let result = evaluate_visual_and_crop_match(
            original_hash,
            rotated_hash,
            None,
            None,
            None,
            None,
            (Some(1000), Some(566)),
            (Some(566), Some(1000)),
            Some(500000),
            Some(500000),
            0.70,
            0.70,
            true,
            false,
            true,  // allow_rotation
            false, // allow_mirroring
        );
        assert!(
            result.is_some(),
            "180° rotated image should be matched when rotation is enabled"
        );
        let match_result = result.unwrap();
        assert!(
            (match_result.score - 1.0).abs() < f64::EPSILON,
            "Rotated match score should be 1.0"
        );
        assert!(
            match_result.reasons.contains(&"rotation_mirroring_checked".to_string()),
            "Reasons should mention rotation/mirroring was checked"
        );
    }

    #[test]
    fn test_independent_thresholds_visual_passes_crop_fails() {
        // When visual and crop thresholds are independent, a score between them
        // should pass visual but not crop.
        let casa_hash: u64 = 0x4777ac2327478753;

        // With visual threshold at 0.50, identical hashes should pass
        let result = evaluate_visual_and_crop_match(
            casa_hash,
            casa_hash,
            None,
            None,
            None,
            None,
            (Some(1000), Some(566)),
            (Some(1000), Some(566)),
            Some(500000),
            Some(500000),
            0.50,  // visual threshold (low)
            0.99,  // crop threshold (high, but irrelevant since visual passes first)
            true,
            true,
            false,
            false,
        );
        assert!(
            result.is_some(),
            "Visual match should pass with a low visual threshold even if crop threshold is high"
        );
    }

    #[test]
    fn test_real_olka_sample_files() {
        use crate::feature::duplicates::fingerprints::{
            compute_multiscale_dhash_256, compute_perceptual_dhash_64, compute_spatial_block_hash,
        };
        use std::path::Path;

        let sample_directory_path = Path::new("../file-samples/Arquivos para testes/Image/png");
        let original_image_path = sample_directory_path.join("olka 2.png");

        if !original_image_path.exists() {
            println!("Sample files not found at {:?}, skipping test", original_image_path);
            return;
        }

        let original_image = image::open(&original_image_path).expect("Failed to open olka 2.png");
        let original_hash = compute_perceptual_dhash_64(&original_image);
        let original_multiscale = compute_multiscale_dhash_256(&original_image);
        let original_spatial_block = compute_spatial_block_hash(&original_image);
        let original_dimensions = (
            Some(original_image.width() as i32),
            Some(original_image.height() as i32),
        );
        let original_file_size = std::fs::metadata(&original_image_path)
            .ok()
            .map(|metadata| metadata.len() as i64);

        println!("\n=== Real Sample Olka Files Evaluation ===");
        println!(
            "Original olka 2.png: hash=0x{:016x}, dims={:?}",
            original_hash, original_dimensions
        );


        let test_file_names = [
            ("olka 4.png", "Cópia exata"),
            ("olka 5.png", "Rotacionado 180º"),
            ("olka 6.png", "Flip vertical"),
            ("olka 7.png", "Flip horizontal"),
            ("olka.png", "Original com uma linha desenhada"),
            ("olka 3.png", "Recortada"),
        ];

        for (sample_file_name, _sample_description) in test_file_names {
            let sample_file_path = sample_directory_path.join(sample_file_name);
            if !sample_file_path.exists() {
                println!("File {:?} does not exist, skipping", sample_file_path);
                continue;
            }

            let sample_image = image::open(&sample_file_path).expect("Failed to open sample image");
            let sample_hash = compute_perceptual_dhash_64(&sample_image);
            let sample_multiscale = compute_multiscale_dhash_256(&sample_image);
            let sample_spatial_block = compute_spatial_block_hash(&sample_image);
            let sample_dimensions = (
                Some(sample_image.width() as i32),
                Some(sample_image.height() as i32),
            );
            let sample_file_size = std::fs::metadata(&sample_file_path)
                .ok()
                .map(|metadata| metadata.len() as i64);

            let direct_similarity = calculate_visual_similarity(original_hash, sample_hash);
            let (crop_two_by_two, _crop_three_by_three) =
                calculate_crop_subgrid_scores(&original_spatial_block, &sample_spatial_block);

            let variant_similarity_rotation = calculate_visual_similarity_with_variants(
                original_hash,
                sample_hash,
                true,
                false,
            );
            let variant_similarity_mirror = calculate_visual_similarity_with_variants(
                original_hash,
                sample_hash,
                false,
                true,
            );

            let evaluated_match = evaluate_visual_and_crop_match(
                original_hash,
                sample_hash,
                Some(&original_multiscale),
                Some(&sample_multiscale),
                Some(&original_spatial_block),
                Some(&sample_spatial_block),
                original_dimensions,
                sample_dimensions,
                original_file_size,
                sample_file_size,
                0.70,
                0.70,
                true,
                true,
                true,
                true,
            );

            if sample_file_name == "olka 4.png" {
                assert!(
                    direct_similarity >= 0.99,
                    "Exact copy must match original with >= 99% similarity"
                );
                assert!(evaluated_match.is_some(), "Exact copy must be accepted");
            } else if sample_file_name == "olka 5.png" {
                assert!(
                    variant_similarity_rotation >= 0.90,
                    "180° rotated image must match with >= 90% similarity under rotation variants"
                );
                assert!(evaluated_match.is_some(), "180° rotated image must be accepted");
            } else if sample_file_name == "olka 6.png" {
                assert!(
                    variant_similarity_mirror >= 0.99,
                    "Vertically flipped image must match with >= 99% similarity under mirror variants"
                );
                assert!(evaluated_match.is_some(), "Vertically flipped image must be accepted");
            } else if sample_file_name == "olka 7.png" {
                assert!(
                    variant_similarity_mirror >= 0.90,
                    "Horizontally flipped image must match with >= 90% similarity under mirror variants"
                );
                assert!(evaluated_match.is_some(), "Horizontally flipped image must be accepted");
            } else if sample_file_name == "olka.png" {
                assert!(
                    direct_similarity >= 0.99,
                    "Original with a drawn line must match with >= 99% similarity"
                );
                assert!(evaluated_match.is_some(), "Image with drawn line must be accepted");
            } else if sample_file_name == "olka 3.png" {
                assert!(
                    direct_similarity >= 0.65,
                    "Cropped image must show partial visual similarity >= 65%"
                );
                assert!(
                    crop_two_by_two >= 0.55,
                    "Cropped image must show subgrid overlap >= 55%"
                );
            }
        }
    }
}

