#[test]
fn check_ffmpeg_kdc() {
    let fixture = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/ffmpeg_kdc.jpg");
    let img = image::open(fixture).unwrap();
    let img_rgb = img.to_rgb8();

    let mut black_pixels = 0;
    let mut white_pixels = 0;
    let mut other_pixels = 0;

    for pixel in img_rgb.pixels() {
        let [r, g, b] = pixel.0;
        if r < 10 && g < 10 && b < 10 {
            black_pixels += 1;
        } else if r > 245 && g > 245 && b > 245 {
            white_pixels += 1;
        } else {
            other_pixels += 1;
        }
    }

    let total = black_pixels + white_pixels + other_pixels;
    println!("Total pixels: {}", total);
    println!(
        "Black pixels: {} ({:.2}%)",
        black_pixels,
        (black_pixels as f64 / total as f64) * 100.0
    );
    println!(
        "White pixels: {} ({:.2}%)",
        white_pixels,
        (white_pixels as f64 / total as f64) * 100.0
    );
    println!(
        "Other pixels: {} ({:.2}%)",
        other_pixels,
        (other_pixels as f64 / total as f64) * 100.0
    );
}
