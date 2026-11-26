//! Windows development host launcher with inherited binary standard streams.
use std::{
    io::{self, ErrorKind},
    os::windows::process::CommandExt,
    path::Path,
    process::{Command, ExitCode},
};

fn launch() -> io::Result<u8> {
    let configuration = std::env::current_exe()?.with_extension("paths");
    let metadata = std::fs::metadata(&configuration)?;
    if !metadata.is_file() || metadata.len() > 16 * 1024 {
        return Err(io::Error::new(
            ErrorKind::InvalidData,
            "Invalid launcher configuration",
        ));
    }
    let paths = std::fs::read_to_string(configuration)?;
    let mut lines = paths.lines();
    let node = lines.next().filter(|value| Path::new(value).is_absolute());
    let script = lines.next().filter(|value| Path::new(value).is_absolute());
    match (node, script, lines.next()) {
        (Some(node), Some(script), None) => {
            let result = Command::new(node)
                .arg(script)
                .creation_flags(0x0800_0000)
                .status()?;
            Ok(if result.success() { 0 } else { 1 })
        }
        _ => Err(io::Error::new(
            ErrorKind::InvalidData,
            "Invalid launcher paths",
        )),
    }
}

fn main() -> ExitCode {
    match launch() {
        Ok(code) => ExitCode::from(code),
        Err(_) => {
            eprintln!("Mundam native bridge could not start.");
            ExitCode::FAILURE
        }
    }
}
