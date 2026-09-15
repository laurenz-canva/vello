// Copyright 2025 the Vello Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#![allow(missing_docs, reason = "Not needed for benchmarks")]
#![allow(dead_code, reason = "Might be unused on platforms not supporting SIMD")]

pub mod allocations;
pub mod data;
#[cfg(feature = "full-benchmarks")]
pub mod allocator;
#[cfg(feature = "full-benchmarks")]
pub mod fine;
#[cfg(feature = "full-benchmarks")]
pub mod flatten;
#[cfg(feature = "full-benchmarks")]
pub mod glyph;
#[cfg(feature = "full-benchmarks")]
pub mod integration;
#[cfg(feature = "full-benchmarks")]
pub mod pixmap;
#[cfg(feature = "full-benchmarks")]
pub mod sort;
#[cfg(feature = "full-benchmarks")]
pub mod strip;
#[cfg(feature = "full-benchmarks")]
pub mod tile;

#[cfg(feature = "full-benchmarks")]
pub(crate) const SEED: [u8; 32] = [0; 32];
#[cfg(feature = "full-benchmarks")]
pub(crate) const EXTENDED: bool = cfg!(feature = "extended");
