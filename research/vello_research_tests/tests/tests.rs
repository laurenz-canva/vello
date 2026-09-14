// Copyright 2026 the Vello Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Single-process entry point for the research renderer tests.
//!
//! Keeping these modules in one test binary allows their shared GPU context and renderer pool to
//! survive between test cases. Cargo would otherwise build each source file as an independent test
//! binary, which would recreate the GPU state once per file even when using libtest.

mod compare_gpu_cpu;
mod emoji;
mod hinting;
mod known_issues;
mod property;
mod regression;
mod smoke_snapshots;
mod snapshot_test_scenes;
