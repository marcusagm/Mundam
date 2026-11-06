fn main() {
    tauri_build::build();

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let resource_name = match std::env::var("CARGO_CFG_TARGET_ENV").as_deref() {
            Ok("msvc") => "resource.lib",
            Ok("gnu" | "gnullvm") => "libresource.a",
            _ => panic!("Unsupported Windows target environment"),
        };
        let output_directory = std::path::PathBuf::from(
            std::env::var_os("OUT_DIR").expect("OUT_DIR must be set by Cargo"),
        );
        // Tauri links this resource to app binaries, but not to test targets.
        println!(
            "cargo:rustc-link-search=native={}",
            output_directory.display()
        );
        println!(
            "cargo:rustc-link-arg-tests={}",
            output_directory.join(resource_name).display()
        );
    }
}
