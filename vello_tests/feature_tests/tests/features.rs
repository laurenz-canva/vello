// Copyright 2026 the Vello Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Compare the common probe scene under the selected shader feature combination.

#[cfg(any(not(target_arch = "wasm32"), feature = "webgl"))]
mod support;

#[cfg(all(target_arch = "wasm32", feature = "webgl"))]
wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn probe_reference() {
    let elements = support::elements();
    let actual = support::render_cpu(&elements);
    support::check_snapshot(actual, "cpu_reference", 1, true);
}

#[cfg(all(not(target_arch = "wasm32"), feature = "wgpu"))]
#[test]
fn probe_wgpu() {
    let elements = support::elements();
    for use_depth_buffer in [true, false] {
        let actual = support::wgpu::render(&elements, use_depth_buffer);
        let run = if use_depth_buffer {
            "wgpu"
        } else {
            "wgpu_no_depth"
        };
        support::check_snapshot(actual, run, 3, false);
    }
}

#[cfg(all(target_arch = "wasm32", feature = "webgl"))]
#[wasm_bindgen_test::wasm_bindgen_test]
fn probe_webgl() {
    let elements = support::elements();
    for use_depth_buffer in [true, false] {
        let actual = support::webgl::render(&elements, use_depth_buffer);
        let run = if use_depth_buffer {
            "webgl"
        } else {
            "webgl_no_depth"
        };
        support::check_snapshot(actual, run, 3, false);
    }
}
