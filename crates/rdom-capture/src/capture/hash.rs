//! The grid hash: a stable fingerprint of what a grid shows, the same on
//! every platform and in every process, so two captures can be compared
//! without shipping both grids.
//!
//! FNV-1a (64-bit) over the grid's compact JSON — no dependency, no seed,
//! and fast enough for every frame of a film. It detects change; it is not
//! a security boundary.

use super::Grid;

/// The grid hash of `grid`: `"fnv1a64:"` and 16 hex digits.
pub fn grid_hash(grid: &Grid) -> String {
    let bytes = serde_json::to_vec(grid).expect("a grid always serializes");
    format!("fnv1a64:{:016x}", fnv1a64(&bytes))
}

/// FNV-1a, 64-bit.
pub fn fnv1a64(bytes: &[u8]) -> u64 {
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    bytes
        .iter()
        .fold(OFFSET, |h, b| (h ^ u64::from(*b)).wrapping_mul(PRIME))
}
