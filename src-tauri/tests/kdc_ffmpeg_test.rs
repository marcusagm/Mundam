use mundam_lib::processing::media::extractors::image::*;
use std::path::Path;

#[test]
fn run() {
    let path = Path::new("/Users/marcusmaia/Documents/Desenvolvimento/Mundam/file-samples/Arquivos para testes/Image/kdc/P003911.KDC");
    let res = generate_ffmpeg_image_preview(path);
    match res {
        Ok((data, _)) => {
            println!("FFmpeg extracted {} bytes", data.len());
        }
        Err(e) => println!("FFmpeg failed: {:?}", e),
    }
}
