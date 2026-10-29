use mundam_lib::processing::media::extractors::image::*;
use std::path::PathBuf;

#[test]
fn test_kdc_flashraw_pipeline() {
    let sample_directory = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../file-samples/Arquivo para testes/Image/kdc");
    let sample_path = sample_directory.join("P003911.KDC");
    let thumbnail_bytes = generate_raw_thumbnail(&sample_path, 512);
    assert!(
        thumbnail_bytes.is_ok(),
        "KDC thumbnail: {thumbnail_bytes:?}"
    );
}
