//! WebAssembly interpreter with an optional bounded preview capability.

use crate::{MAX_MODULE_BYTES, Outcome, preview::PreviewRead};
use std::io;
use wasmi::{Caller, Extern, StoreLimits};

const PREVIEW_IMPORT: &str = "mundam:probe/preview@0.1.0";
const PREVIEW_READ: &str = "read";

struct State<ReadPreview> {
    limits: StoreLimits,
    preview: Option<(u32, ReadPreview)>,
}

/// Interprets a single module with no host imports and one typed export.
pub fn evaluate(bytes: &[u8]) -> Outcome {
    execute::<fn(PreviewRead) -> io::Result<Vec<u8>>>(bytes, None)
}

pub(crate) fn execute<ReadPreview>(bytes: &[u8], preview: Option<(u32, ReadPreview)>) -> Outcome
where
    ReadPreview: FnMut(PreviewRead) -> io::Result<Vec<u8>> + 'static,
{
    use wasmi::{Config, Engine, Linker, Module, Store, StoreLimitsBuilder, TrapCode};

    if bytes.len() > MAX_MODULE_BYTES {
        return Outcome::InvalidModule;
    }
    let mut config = Config::default();
    config
        .consume_fuel(true)
        .allow_start_fn(false)
        .set_max_recursion_depth(64)
        .set_max_stack_height(256 * 1024);
    let engine = Engine::new(&config);
    let Ok(module) = Module::new(&engine, bytes) else {
        return Outcome::InvalidModule;
    };
    if module.imports().any(|import| {
        preview.is_none() || import.module() != PREVIEW_IMPORT || import.name() != PREVIEW_READ
    }) {
        return Outcome::ImportsDenied;
    }

    let limits = StoreLimitsBuilder::new()
        .instances(1)
        .memories(1)
        .memory_size(2 * 64 * 1024)
        .tables(1)
        .table_elements(32)
        .trap_on_grow_failure(true)
        .build();
    let mut linker = Linker::new(&engine);
    if preview.is_some()
        && linker
            .func_wrap(PREVIEW_IMPORT, PREVIEW_READ, read_preview::<ReadPreview>)
            .is_err()
    {
        return Outcome::GuestTrap;
    }
    let mut store = Store::new(&engine, State { limits, preview });
    store.limiter(|state| &mut state.limits);
    if store.set_fuel(100_000).is_err() {
        return Outcome::GuestTrap;
    }
    let Ok(instance) = linker.instantiate_and_start(&mut store, &module) else {
        return Outcome::ResourceLimit;
    };
    let Ok(entry) = instance.get_typed_func::<(), i32>(&store, "run") else {
        return Outcome::InvalidModule;
    };
    match entry.call(&mut store, ()) {
        Ok(result) => Outcome::Completed(result),
        Err(error) => match error.as_trap_code() {
            Some(TrapCode::OutOfFuel) => Outcome::FuelExhausted,
            Some(TrapCode::GrowthOperationLimited | TrapCode::StackOverflow) => {
                Outcome::ResourceLimit
            }
            _ => Outcome::GuestTrap,
        },
    }
}

fn read_preview<ReadPreview>(
    mut caller: Caller<'_, State<ReadPreview>>,
    offset: u32,
    length: u32,
    destination: u32,
) -> Result<i32, wasmi::Error>
where
    ReadPreview: FnMut(PreviewRead) -> io::Result<Vec<u8>>,
{
    let Some(Extern::Memory(memory)) = caller.get_export("memory") else {
        return Err(wasmi::Error::new("preview guest has no exported memory"));
    };
    let request = PreviewRead { offset, length };
    let memory_size = memory.data_size(&caller);
    let Some((byte_count, read)) = &mut caller.data_mut().preview else {
        return Err(wasmi::Error::new("preview unavailable"));
    };
    if !request.fits(*byte_count)
        || destination
            .checked_add(length)
            .is_none_or(|end| end as usize > memory_size)
    {
        return Err(wasmi::Error::new(
            "invalid preview range or memory destination",
        ));
    }
    let bytes = read(request).map_err(|_| wasmi::Error::new("preview read failed"))?;
    if bytes.len() != length as usize {
        return Err(wasmi::Error::new("incomplete preview reply"));
    }
    memory
        .write(&mut caller, destination as usize, &bytes)
        .map_err(|_| wasmi::Error::new("invalid preview memory destination"))?;
    Ok(length as i32)
}
