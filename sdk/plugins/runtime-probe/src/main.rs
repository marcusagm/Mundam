//! Runs one request over the helper's inherited pipes.

use std::io;

fn main() -> io::Result<()> {
    mundam_runtime_probe::child::serve(io::stdin(), io::stdout())
}
