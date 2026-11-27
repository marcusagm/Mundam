//! Engine-independent process supervision and pipe protocol for the runtime proof.

#[cfg(windows)]
pub mod job;
pub mod preview;
pub mod supervised;

/// Byte cap checked at both ends of the probe's anonymous pipe.
pub const MAX_MODULE_BYTES: usize = 256 * 1024;

/// Binary prefix for WebAssembly Component Model version 0x0d, layer 1.
pub const COMPONENT_HEADER: &[u8; 8] = b"\0asm\x0d\0\x01\0";

/// Typed failures returned by the preview conformance component.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreviewRejection {
    Unavailable,
    InvalidRange,
    BudgetExhausted,
}

/// The child emits a fixed-size response. Timeouts and cancellation are host outcomes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Completed(i32),
    ComponentCompleted([u8; 8]),
    PreviewRejected(PreviewRejection),
    InvalidModule,
    ImportsDenied,
    FuelExhausted,
    ResourceLimit,
    GuestTrap,
    TimedOut,
    Cancelled,
}

/// Encodes only outcomes that may originate from the child.
pub fn encode(outcome: Outcome) -> Option<[u8; 5]> {
    let (status, result) = match outcome {
        Outcome::Completed(result) => (0, result),
        Outcome::InvalidModule => (1, 0),
        Outcome::ImportsDenied => (2, 0),
        Outcome::FuelExhausted => (3, 0),
        Outcome::ResourceLimit => (4, 0),
        Outcome::GuestTrap => (5, 0),
        Outcome::PreviewRejected(PreviewRejection::Unavailable) => (6, 0),
        Outcome::PreviewRejected(PreviewRejection::InvalidRange) => (7, 0),
        Outcome::PreviewRejected(PreviewRejection::BudgetExhausted) => (8, 0),
        Outcome::ComponentCompleted(_) | Outcome::TimedOut | Outcome::Cancelled => return None,
    };
    let mut response = [0; 5];
    response[0] = status;
    response[1..].copy_from_slice(&result.to_le_bytes());
    Some(response)
}

fn decode([status, first, second, third, fourth]: [u8; 5], component: bool) -> Option<Outcome> {
    let value = i32::from_le_bytes([first, second, third, fourth]);
    match (status, value) {
        (0, value) if !component => Some(Outcome::Completed(value)),
        (1, 0) => Some(Outcome::InvalidModule),
        (2, 0) => Some(Outcome::ImportsDenied),
        (3, 0) => Some(Outcome::FuelExhausted),
        (4, 0) => Some(Outcome::ResourceLimit),
        (5, 0) => Some(Outcome::GuestTrap),
        (6, 0) if component => Some(Outcome::PreviewRejected(PreviewRejection::Unavailable)),
        (7, 0) if component => Some(Outcome::PreviewRejected(PreviewRejection::InvalidRange)),
        (8, 0) if component => Some(Outcome::PreviewRejected(PreviewRejection::BudgetExhausted)),
        _ => None,
    }
}
