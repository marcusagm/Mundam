//! Framing and limits shared by the isolated helper and its supervisor.

use std::io;

/// Header identifying the mediated form of the request.
pub const MODE: [u8; 4] = *b"MPR1";
/// Mediated request carrying a Component Model binary.
pub const COMPONENT_MODE: [u8; 4] = *b"MPC1";
/// Child message announcing its terminal outcome.
pub const FINISHED: u8 = 0;
/// Child message requesting one range.
pub const READ: u8 = 1;
/// Terminal component signature with eight bytes of inline payload.
pub const COMPONENT_RESULT: u8 = 2;
/// Host reply announcing exactly the requested bytes.
pub const READ_OK: u8 = 0;
pub(crate) const MAX_BYTES: u32 = 1024 * 1024;
pub(crate) const MAX_CHUNK: u32 = 64 * 1024;
const MAX_CALLS: u32 = 32;

/// Range for the single preview selected by the host for this attempt.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct PreviewRead {
    pub offset: u32,
    pub length: u32,
}

impl PreviewRead {
    pub(crate) fn from_frame(frame: [u8; 8]) -> Self {
        let [a, b, c, d, e, f, g, h] = frame;
        Self {
            offset: u32::from_le_bytes([a, b, c, d]),
            length: u32::from_le_bytes([e, f, g, h]),
        }
    }

    /// Encodes the range in the child-to-host read frame.
    pub fn frame(self) -> [u8; 8] {
        let mut frame = [0; 8];
        frame[..4].copy_from_slice(&self.offset.to_le_bytes());
        frame[4..].copy_from_slice(&self.length.to_le_bytes());
        frame
    }

    /// Checks the range against the offered byte count and per-request cap.
    pub fn fits(self, byte_count: u32) -> bool {
        (1..=MAX_CHUNK).contains(&self.length)
            && self
                .offset
                .checked_add(self.length)
                .is_some_and(|end| end <= byte_count)
    }
}

/// Bounds both the number of calls and aggregate bytes in one attempt.
pub struct PreviewBudget {
    byte_count: u32,
    calls: u32,
    bytes: u32,
}

impl PreviewBudget {
    /// Creates a budget for an offered preview.
    ///
    /// # Errors
    /// Rejects empty or oversized offers.
    pub fn new(byte_count: u32) -> io::Result<Self> {
        if !(1..=MAX_BYTES).contains(&byte_count) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid preview size",
            ));
        }
        Ok(Self {
            byte_count,
            calls: 0,
            bytes: 0,
        })
    }

    /// Reserves one request before consulting the host or writing to the pipe.
    ///
    /// # Errors
    /// Rejects invalid ranges and exhausted budgets.
    pub fn reserve(&mut self, read: PreviewRead) -> io::Result<()> {
        if !read.fits(self.byte_count)
            || self.calls >= MAX_CALLS
            || self.bytes + read.length > MAX_BYTES
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid or exhausted preview read",
            ));
        }
        self.calls += 1;
        self.bytes += read.length;
        Ok(())
    }
}
