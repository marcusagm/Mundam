fn main() {
    // Ensure native MinGW binary directory is prioritized over MSYS POSIX (/usr/bin) in PATH.
    // MSYS's /usr/bin/windres treats Windows path backslashes as escape characters, breaking resource compilation.
    #[cfg(target_os = "windows")]
    if let Ok(current_path) = std::env::var("PATH") {
        let new_path = format!(
            "C:\\msys64\\ucrt64\\bin;C:\\msys64\\mingw64\\bin;{}",
            current_path
        );
        std::env::set_var("PATH", new_path);
    }

    tauri_build::build();
}
