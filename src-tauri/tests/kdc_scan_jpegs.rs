use std::path::Path;

#[test]
fn scan_all_jpegs() {
    let p = Path::new("/Users/marcusmaia/Documents/Desenvolvimento/Mundam/file-samples/Arquivos para testes/Image/kdc/RAW_KODAK_DC120_WITH_JPEG.KDC");
    let memory_map = std::fs::read(p).unwrap();

    let mut scan_offset = 0;
    while scan_offset < memory_map.len() - 3 {
        if memory_map[scan_offset] == 0xFF
            && memory_map[scan_offset + 1] == 0xD8
            && memory_map[scan_offset + 2] == 0xFF
        {
            println!("Found JPEG marker at {}", scan_offset);

            // let's try to parse it with image::load_from_memory
            if let Ok(img) = image::load_from_memory(&memory_map[scan_offset..]) {
                println!(
                    "Valid JPEG! Width: {}, Height: {}",
                    img.width(),
                    img.height()
                );
            } else {
                println!("Invalid or truncated JPEG.");
            }
        }
        scan_offset += 1;
    }
}
