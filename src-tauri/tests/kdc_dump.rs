use mundam_lib::processing::media::extractors::image::*;
use std::path::Path;

#[test]
fn dump() {
    let p = Path::new("/Users/marcusmaia/Documents/Desenvolvimento/Mundam/file-samples/Arquivos para testes/Image/kdc/P003911.KDC");
    let (bytes, _) = brute_force_extract_jpeg_bytes(p).unwrap();
    std::fs::write("dump.jpg", &bytes).unwrap();
}
