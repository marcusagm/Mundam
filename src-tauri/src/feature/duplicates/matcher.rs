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
    let hamming_distance = (hash_one ^ hash_two).count_ones();
    1.0 - (hamming_distance as f64 / 64.0)
}

/// Calculates multi-scale similarity between two 256-bit hexadecimal hash strings.
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
    for chunk_index in 0..4 {
        let start_index = chunk_index * 16;
        let end_index = (chunk_index + 1) * 16;
        let slice_one = &multi_scale_hex_one[start_index..end_index];
        let slice_two = &multi_scale_hex_two[start_index..end_index];

        let value_one = u64::from_str_radix(slice_one, 16).unwrap_or(0);
        let value_two = u64::from_str_radix(slice_two, 16).unwrap_or(0);
        total_distance += (value_one ^ value_two).count_ones();
    }

    1.0 - (total_distance as f64 / 256.0)
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
    if block_hash_one.len() < 64 || block_hash_two.len() < 64 {
        return 0.0;
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

    score_two_by_two_forward
        .max(score_two_by_two_backward)
        .max(score_three_by_three_forward)
        .max(score_three_by_three_backward)
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
/// * `minimum_score_threshold` - Minimum confidence score required to form a match.
/// * `consider_crop_match` - Whether crop matching is enabled in the active rule set.
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
    minimum_score_threshold: f64,
    consider_crop_match: bool,
) -> Option<DuplicateMatchResult> {
    let visual_similarity = calculate_visual_similarity(hash_one, hash_two);

    let multiscale_similarity = match (multi_scale_one, multi_scale_two) {
        (Some(first_hex), Some(second_hex)) => {
            calculate_multiscale_similarity(first_hex, second_hex)
        }
        _ => visual_similarity,
    };

    let crop_similarity = match (block_hash_one, block_hash_two) {
        (Some(first_block), Some(second_block)) => {
            calculate_crop_similarity(first_block, second_block)
        }
        _ => 0.0,
    };

    // Primary check: Direct visual/perceptual match
    if visual_similarity >= minimum_score_threshold {
        let mut explainable_reasons: Vec<String> = Vec::new();
        let percentage_formatted = (visual_similarity * 100.0).round() as u32;
        explainable_reasons.push(format!("visual_similarity:{}%", percentage_formatted));

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

    // Secondary check: Crop or partial derivative match
    if consider_crop_match && crop_similarity >= minimum_score_threshold {
        let mut explainable_reasons: Vec<String> = Vec::new();
        let crop_percentage = (crop_similarity * 100.0).round() as u32;
        explainable_reasons.push(format!("crop_detected:{}%", crop_percentage));

        if multiscale_similarity >= minimum_score_threshold {
            let multiscale_percentage = (multiscale_similarity * 100.0).round() as u32;
            explainable_reasons.push(format!("multiscale_similarity:{}%", multiscale_percentage));
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
            score: crop_similarity,
            reasons: explainable_reasons,
        });
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
}
