use mundam_lib::processing::media::extractors::kdc::*;
use std::path::Path;

#[test]
fn test_kdc_preview() {
    let paths = [
        "/Users/marcusmaia/Documents/Desenvolvimento/Mundam/file-samples/Arquivos para testes/Image/kdc/P003911.KDC",
        "/Users/marcusmaia/Documents/Desenvolvimento/Mundam/file-samples/Arquivos para testes/Image/kdc/RAW_KODAK_DC120_WITH_JPEG.KDC",
        "/Users/marcusmaia/Documents/Desenvolvimento/Mundam/file-samples/Arquivos para testes/Image/kdc/RAW__KODAK_EASYSHARE_Z1015-IS.KDC",
    ];
    for p in paths {
        let path = Path::new(p);
        if !path.exists() {
            println!("File not found: {:?}", p);
            continue;
        }
        println!("Testing: {:?}", p);
        match extract_kdc_preview(path) {
            Ok(bytes) => println!("Success! Extracted {} bytes", bytes.len()),
            Err(e) => println!("Failed to extract preview: {:?}", e),
        }
    }
}
