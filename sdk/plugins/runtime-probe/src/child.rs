//! Helper-side pipe exchange. Only the interpreter can invoke a preview read.

use crate::{
    MAX_MODULE_BYTES, Outcome, encode, evaluate, interpreter,
    preview::{
        COMPONENT_MODE, COMPONENT_RESULT, FINISHED, MODE, PreviewBudget, PreviewRead, READ, READ_OK,
    },
};
use std::{
    io::{self, Read, Write},
    sync::{Arc, Mutex},
};

/// Serves one request using the helper's inherited input and output.
///
/// # Errors
/// Returns malformed framing, truncated requests and pipe failures.
pub fn serve(
    mut input: impl Read + Send + 'static,
    output: impl Write + Send + 'static,
) -> io::Result<()> {
    let mut header = [0; 4];
    input.read_exact(&mut header)?;
    let component = header == COMPONENT_MODE;
    let mediated = component || header == MODE;
    let (module_length, preview) = if mediated {
        input.read_exact(&mut header)?;
        let module_length = u32::from_le_bytes(header);
        input.read_exact(&mut header)?;
        let byte_count = u32::from_le_bytes(header);
        (
            module_length,
            Some((byte_count, PreviewBudget::new(byte_count)?)),
        )
    } else {
        (u32::from_le_bytes(header), None)
    };
    if module_length as usize > MAX_MODULE_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "oversized module",
        ));
    }
    let mut module = vec![0; module_length as usize];
    input.read_exact(&mut module)?;
    let output = Arc::new(Mutex::new(output));
    let outcome = if let Some((byte_count, mut budget)) = preview {
        let replies = Arc::clone(&output);
        let mut read = move |request| {
            let mut output = replies
                .lock()
                .map_err(|_| io::Error::other("reply pipe lock poisoned"))?;
            read_preview(&mut input, &mut *output, request)
        };
        if component {
            execute_component(&module, byte_count, budget, read)
        } else {
            interpreter::execute(
                &module,
                Some((byte_count, move |request| {
                    budget.reserve(request)?;
                    read(request)
                })),
            )
        }
    } else {
        if input.read(&mut [0])? != 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "unexpected input",
            ));
        }
        evaluate(&module)
    };
    let mut output = output
        .lock()
        .map_err(|_| io::Error::other("reply pipe lock poisoned"))?;
    if let Outcome::ComponentCompleted(signature) = outcome {
        output.write_all(&[COMPONENT_RESULT])?;
        output.write_all(&signature)?;
    } else {
        if mediated {
            output.write_all(&[FINISHED])?;
        }
        let response = encode(outcome).ok_or_else(|| io::Error::other("invalid child outcome"))?;
        output.write_all(&response)?;
    }
    output.flush()
}

#[cfg(feature = "component-proof")]
fn execute_component(
    module: &[u8],
    byte_count: u32,
    budget: PreviewBudget,
    read: impl FnMut(PreviewRead) -> io::Result<Vec<u8>> + Send + 'static,
) -> Outcome {
    crate::component::execute(module, byte_count, budget, read)
}

#[cfg(not(feature = "component-proof"))]
fn execute_component(
    _: &[u8],
    _: u32,
    _: PreviewBudget,
    _: impl FnMut(PreviewRead) -> io::Result<Vec<u8>>,
) -> Outcome {
    Outcome::InvalidModule
}

fn read_preview(
    input: &mut impl Read,
    output: &mut impl Write,
    request: PreviewRead,
) -> io::Result<Vec<u8>> {
    output.write_all(&[READ])?;
    output.write_all(&request.frame())?;
    output.flush()?;
    let mut status = [0; 1];
    input.read_exact(&mut status)?;
    if status[0] != READ_OK {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid preview reply tag",
        ));
    }
    let mut bytes = vec![0; request.length as usize];
    input.read_exact(&mut bytes)?;
    Ok(bytes)
}
