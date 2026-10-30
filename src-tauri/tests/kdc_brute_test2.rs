use mundam_lib::processing::media::extractors::image::*;
use std::path::Path;

#[test]
fn run() {
    let p = Path::new("/Users/marcusmaia/Documents/Desenvolvimento/Mundam/file-samples/Arquivos para testes/Image/kdc/RAW_KODAK_DC120_WITH_JPEG.KDC");
    let (bytes, _) = brute_force_extract_jpeg_bytes(p).unwrap();
    println!("Extracted {} bytes from WITH_JPEG", bytes.len());
}
