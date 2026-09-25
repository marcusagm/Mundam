use image::{DynamicImage, GenericImageView};

/// Computes a standard 64-bit gradient difference hash (dHash) for global perceptual similarity.
///
/// Converts the image to grayscale and resizes it to 9x8 pixels to compute 64 horizontal
/// luminance comparisons. This provides a rotation-sensitive, recompression-resilient fingerprint.
///
/// # Arguments
/// * `image_reference` - The dynamic image to hash.
///
/// # Returns
/// A 64-bit unsigned integer representing the gradient difference hash.
pub fn compute_perceptual_dhash_64(image_reference: &DynamicImage) -> u64 {
    let grayscale_image = image_reference.grayscale();
    let resized_image = image::imageops::resize(
        &grayscale_image,
        9,
        8,
        image::imageops::FilterType::Nearest,
    );

    let mut hash_value: u64 = 0;
    for row_index in 0..8 {
        for column_index in 0..8 {
            let pixel_left = resized_image.get_pixel(column_index, row_index)[0];
            let pixel_right = resized_image.get_pixel(column_index + 1, row_index)[0];
            hash_value <<= 1;
            if pixel_left > pixel_right {
                hash_value |= 1;
            }
        }
    }
    hash_value
}

/// Computes a multi-scale 256-bit gradient difference hash (17x16 pixels) for fine-grained
/// texture analysis and severe derivative detection across varying scales.
///
/// # Arguments
/// * `image_reference` - The dynamic image to hash.
///
/// # Returns
/// A 64-character hexadecimal string representing the 256-bit hash.
pub fn compute_multiscale_dhash_256(image_reference: &DynamicImage) -> String {
    let grayscale_image = image_reference.grayscale();
    let resized_image = image::imageops::resize(
        &grayscale_image,
        17,
        16,
        image::imageops::FilterType::Nearest,
    );

    let mut hex_string_buffer = String::with_capacity(64);
    for chunk_index in 0..4 {
        let mut chunk_hash_value: u64 = 0;
        let start_row_index = chunk_index * 4;
        let end_row_index = (chunk_index + 1) * 4;

        for row_index in start_row_index..end_row_index {
            for column_index in 0..16 {
                let pixel_left = resized_image.get_pixel(column_index, row_index)[0];
                let pixel_right = resized_image.get_pixel(column_index + 1, row_index)[0];
                chunk_hash_value <<= 1;
                if pixel_left > pixel_right {
                    chunk_hash_value |= 1;
                }
            }
        }
        hex_string_buffer.push_str(&format!("{:016x}", chunk_hash_value));
    }
    hex_string_buffer
}

/// Computes a 4x4 spatial grid block hash (16 tiles x 16-bit dHash per tile = 256 bits = 64 hex characters)
/// specifically designed for detecting partial crops, sub-region overlaps, and spatial variations.
///
/// # Arguments
/// * `image_reference` - The dynamic image to hash.
///
/// # Returns
/// A 64-character hexadecimal string representing the 16 local tile hashes.
pub fn compute_spatial_block_hash(image_reference: &DynamicImage) -> String {
    let grayscale_image = image_reference.grayscale();
    let (image_width, image_height) = grayscale_image.dimensions();
    let grid_size: u32 = 4;
    let mut block_hex_buffer = String::with_capacity(64);

    for row_index in 0..grid_size {
        for column_index in 0..grid_size {
            let tile_start_x = (column_index * image_width) / grid_size;
            let tile_start_y = (row_index * image_height) / grid_size;
            let tile_end_x = ((column_index + 1) * image_width) / grid_size;
            let tile_end_y = ((row_index + 1) * image_height) / grid_size;
            let tile_width = (tile_end_x - tile_start_x).max(1);
            let tile_height = (tile_end_y - tile_start_y).max(1);

            let sub_image = image::imageops::crop_imm(
                &grayscale_image,
                tile_start_x,
                tile_start_y,
                tile_width,
                tile_height,
            );

            let resized_tile = image::imageops::resize(
                &sub_image.to_image(),
                5,
                4,
                image::imageops::FilterType::Nearest,
            );

            let mut tile_hash: u16 = 0;
            for tile_row_index in 0..4 {
                for tile_col_index in 0..4 {
                    let pixel_left = resized_tile.get_pixel(tile_col_index, tile_row_index)[0];
                    let pixel_right = resized_tile.get_pixel(tile_col_index + 1, tile_row_index)[0];
                    tile_hash <<= 1;
                    if pixel_left > pixel_right {
                        tile_hash |= 1;
                    }
                }
            }
            block_hex_buffer.push_str(&format!("{:04x}", tile_hash));
        }
    }
    block_hex_buffer
}

/// Helper function to determine if a media format family or format type corresponds to an image.
///
/// Avoids case-sensitivity bugs by performing case-insensitive matching across known image identifiers.
///
/// # Arguments
/// * `format_family` - The family string (e.g., "Image", "image").
/// * `format_type` - The format type string (e.g., "PNG Image", "image/png").
///
/// # Returns
/// `true` if the media should be processed as an image, `false` otherwise.
pub fn is_image_media(format_family: &str, format_type: &str) -> bool {
    let normalized_family = format_family.to_lowercase();
    let normalized_format = format_type.to_lowercase();

    normalized_family == "image"
        || normalized_family.starts_with("image/")
        || normalized_format.contains("image")
        || normalized_format.starts_with("image/")
        || normalized_format == "png"
        || normalized_format == "jpeg"
        || normalized_format == "jpg"
        || normalized_format == "webp"
        || normalized_format == "avif"
        || normalized_format == "gif"
        || normalized_format == "bmp"
}
