//! Runs the WIT conformance world inside the supervised helper.

use crate::{
    Outcome, PreviewRejection,
    preview::{PreviewBudget, PreviewRead},
};
use std::io;
use wasmtime::{
    Config, Engine, Store, StoreLimits, StoreLimitsBuilder, Trap,
    component::{Component, HasSelf, Linker, Resource, ResourceTable, bindgen},
};

const PREVIEW_INTERFACE: &str = "mundam:worker/image-preview@0.1.0";
const MEMORY_BYTES: usize = 4 * 1024 * 1024;
const FUEL: u64 = 1_000_000;

bindgen!({
    path: "../v1/wit",
    world: "preview-check",
    with: { "mundam:worker/image-preview.preview": PreviewToken },
    imports: { default: trappable },
});

use mundam::worker::image_preview::{Host, HostPreview};

pub struct PreviewToken;

struct HostState {
    table: ResourceTable,
    byte_count: u32,
    budget: PreviewBudget,
    read: Box<dyn FnMut(PreviewRead) -> io::Result<Vec<u8>> + Send>,
    limits: StoreLimits,
    active: bool,
    issued: bool,
}

impl Host for HostState {
    fn open_selected_preview(
        &mut self,
    ) -> wasmtime::Result<Result<Resource<PreviewToken>, PreviewError>> {
        if !self.active || self.issued {
            return Ok(Err(PreviewError::Unavailable));
        }
        let preview = self.table.push(PreviewToken)?;
        self.issued = true;
        Ok(Ok(preview))
    }
}

impl HostPreview for HostState {
    fn byte_count(&mut self, preview: Resource<PreviewToken>) -> wasmtime::Result<u32> {
        self.table.get(&preview)?;
        Ok(self.byte_count)
    }

    fn read(
        &mut self,
        preview: Resource<PreviewToken>,
        offset: u32,
        length: u32,
    ) -> wasmtime::Result<Result<Vec<u8>, PreviewError>> {
        self.table.get(&preview)?;
        let request = PreviewRead { offset, length };
        if !request.fits(self.byte_count) {
            return Ok(Err(PreviewError::InvalidRange));
        }
        if self.budget.reserve(request).is_err() {
            return Ok(Err(PreviewError::BudgetExhausted));
        }
        let bytes = (self.read)(request).map_err(|_| wasmtime::Error::msg("broker read failed"))?;
        if bytes.len() != length as usize {
            return Err(wasmtime::Error::msg("broker returned an incomplete range"));
        }
        Ok(Ok(bytes))
    }

    fn drop(&mut self, preview: Resource<PreviewToken>) -> wasmtime::Result<()> {
        self.table.delete(preview)?;
        Ok(())
    }
}

pub(crate) fn execute(
    module: &[u8],
    byte_count: u32,
    budget: PreviewBudget,
    read: impl FnMut(PreviewRead) -> io::Result<Vec<u8>> + Send + 'static,
) -> Outcome {
    let mut config = Config::new();
    config
        .consume_fuel(true)
        .max_wasm_stack(256 * 1024)
        .memory_reservation(MEMORY_BYTES as u64)
        .memory_reservation_for_growth(0)
        .memory_guard_size(64 * 1024);
    let Ok(engine) = Engine::new(&config) else {
        return Outcome::GuestTrap;
    };
    let Ok(component) = Component::new(&engine, module) else {
        return Outcome::InvalidModule;
    };
    let component_type = component.component_type();
    let mut imports = component_type.imports(&engine);
    if imports.next().map(|(name, _)| name) != Some(PREVIEW_INTERFACE) || imports.next().is_some() {
        return Outcome::ImportsDenied;
    }
    let mut linker = Linker::new(&engine);
    if PreviewCheck::add_to_linker::<_, HasSelf<_>>(&mut linker, |state| state).is_err() {
        return Outcome::GuestTrap;
    }
    let Ok(pre) = linker.instantiate_pre(&component) else {
        return Outcome::ImportsDenied;
    };
    let Ok(pre) = PreviewCheckPre::new(pre) else {
        return Outcome::InvalidModule;
    };
    let limits = StoreLimitsBuilder::new()
        .instances(16)
        .memories(4)
        .memory_size(MEMORY_BYTES)
        .tables(8)
        .table_elements(1024)
        .trap_on_grow_failure(true)
        .build();
    let mut store = Store::new(
        &engine,
        HostState {
            table: ResourceTable::new(),
            byte_count,
            budget,
            read: Box::new(read),
            limits,
            active: false,
            issued: false,
        },
    );
    store.limiter(|state| &mut state.limits);
    if store.set_fuel(FUEL).is_err() {
        return Outcome::GuestTrap;
    }
    let instance = match pre.instantiate(&mut store) {
        Ok(instance) => instance,
        Err(error) => return trap_outcome(&error),
    };
    store.data_mut().active = true;
    match instance.call_read_preview_signature(&mut store) {
        Ok(Ok(bytes)) => {
            <[u8; 8]>::try_from(bytes).map_or(Outcome::InvalidModule, Outcome::ComponentCompleted)
        }
        Ok(Err(error)) => Outcome::PreviewRejected(match error {
            PreviewError::Unavailable => PreviewRejection::Unavailable,
            PreviewError::InvalidRange => PreviewRejection::InvalidRange,
            PreviewError::BudgetExhausted => PreviewRejection::BudgetExhausted,
        }),
        Err(error) => trap_outcome(&error),
    }
}

fn trap_outcome(error: &wasmtime::Error) -> Outcome {
    match error.downcast_ref::<Trap>() {
        Some(Trap::OutOfFuel) => Outcome::FuelExhausted,
        Some(Trap::StackOverflow) => Outcome::ResourceLimit,
        _ => Outcome::GuestTrap,
    }
}
