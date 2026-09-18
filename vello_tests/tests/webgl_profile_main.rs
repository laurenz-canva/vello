// Copyright 2026 the Vello Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#![allow(missing_docs, reason = "we don't need docs for testing")]
#![allow(
    dead_code,
    reason = "the profiling target only uses a small subset of the shared test renderer"
)]

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

#[path = "renderer.rs"]
mod renderer;
#[path = "util.rs"]
mod util;
mod webgl_profile;
