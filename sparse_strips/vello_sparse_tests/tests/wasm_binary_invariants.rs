// Copyright 2025 the Vello Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use wasm_bindgen_test::*;

#[cfg(not(target_feature = "simd128"))]
#[wasm_bindgen_test]
async fn no_simd_instruction_inclusion() {
    // Unless the WASM binary is explicitly built with `RUSTFLAGS=-Ctarget-feature=+simd128` then it
    // is imperative that there isn't a single SIMD instruction in the resulting binary. These can
    // accidentally creep into the binary due to usage of `#![cfg(target_feature = "simd128")]`. Any
    // inclusion of a SIMD instruction in a non-SIMD WASM binary can invalidate the whole binary for
    // browsers (or WebAssembly runtimes) that do not have SIMD support.
    //
    // This test runs when simd128 is not enabled, and self-introspects the binary to ensure no SIMD
    // instructions are included.

    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;
    use wasmparser::{Validator, WasmFeatures};

    let window = web_sys::window().unwrap();
    let url = "/wasm-bindgen-test_bg.wasm";
    let response = JsFuture::from(window.fetch_with_str(url)).await.unwrap();
    let response: web_sys::Response = response.dyn_into().unwrap();
    assert!(response.ok(), "binary could not be fetched");
    let buffer = JsFuture::from(response.array_buffer().unwrap())
        .await
        .unwrap();

    let wasm_module_bytes = web_sys::js_sys::Uint8Array::new(&buffer).to_vec();
    let mut wasm_validator_without_simd =
        Validator::new_with_features(WasmFeatures::all().difference(WasmFeatures::SIMD));

    assert!(
        wasm_validator_without_simd
            .validate_all(&wasm_module_bytes)
            .is_ok(),
        "WebAssembly module contains unexpected SIMD instructions"
    );
}

#[cfg(feature = "webgl")]
fn create_canvas(width: u32, height: u32) -> web_sys::HtmlCanvasElement {
    use wasm_bindgen::JsCast;

    let document = web_sys::window().unwrap().document().unwrap();
    let canvas = document
        .create_element("canvas")
        .unwrap()
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .unwrap();
    canvas.set_width(width);
    canvas.set_height(height);
    canvas
}

#[cfg(feature = "webgl")]
async fn assert_probe_succeeds(mut pending: vello_hybrid::WebGlPendingProbe) {
    use vello_hybrid::WebGlProbeStatus;
    use wasm_bindgen_futures::JsFuture;

    async fn wait_for_animation_frame() {
        let promise = web_sys::js_sys::Promise::new(&mut |resolve, _reject| {
            web_sys::window()
                .unwrap()
                .request_animation_frame(&resolve)
                .unwrap();
        });
        JsFuture::from(promise).await.unwrap();
    }

    const MAX_FRAMES: u32 = 600;

    for _ in 0..MAX_FRAMES {
        match pending.try_finish() {
            Ok(WebGlProbeStatus::Complete(result)) => {
                assert!(result.is_success(), "probe failed unexpectedly");
                return;
            }
            Ok(WebGlProbeStatus::Pending(next_pending)) => {
                pending = next_pending;
                wait_for_animation_frame().await;
            }
            Err(error) => panic!("WebGlRenderer::probe() readback failed: {error:?}"),
        }
    }

    panic!(
        "WebGlRenderer::probe() did not finish within {} animation frames",
        MAX_FRAMES
    );
}

#[cfg(feature = "webgl")]
#[wasm_bindgen_test]
async fn webgl_probe_succeeds() {
    let canvas = create_canvas(200, 200);

    let mut renderer = vello_hybrid::WebGlRenderer::new(&canvas);
    let pending = renderer
        .probe()
        .unwrap_or_else(|error| panic!("WebGlRenderer::probe() failed to render: {error:?}"));
    assert_probe_succeeds(pending).await;
}

#[cfg(feature = "webgl")]
#[wasm_bindgen_test]
async fn webgl_probe_succeeds_after_filter_render() {
    use vello_common::filter_effects::{EdgeMode, Filter, FilterPrimitive};
    use vello_common::kurbo::Rect;
    use vello_hybrid::{RenderSize, Resources, Scene, WebGlRenderer};

    let canvas = create_canvas(200, 200);

    let mut renderer = WebGlRenderer::new(&canvas);
    let mut scene = Scene::new(200, 200);
    scene.push_filter_layer(Filter::from_primitive(FilterPrimitive::GaussianBlur {
        std_deviation: 2.0,
        edge_mode: EdgeMode::None,
    }));
    scene.set_paint(vello_common::color::palette::css::REBECCA_PURPLE);
    scene.fill_rect(&Rect::new(20.0, 20.0, 80.0, 80.0));
    scene.pop_layer();

    renderer
        .render(
            &scene,
            &mut Resources::new(),
            &RenderSize {
                width: 200,
                height: 200,
            },
        )
        .expect("filter scene should render");

    let pending = renderer
        .probe()
        .unwrap_or_else(|error| panic!("WebGlRenderer::probe() failed to render: {error:?}"));
    assert_probe_succeeds(pending).await;
}

// This test reproduces a bug where creating a renderer would leave a non-default framebuffer without
// depth attachment bound, as a result of which `DEPTH_BITS` would return 0 when creating a second
// renderer.
#[cfg(feature = "webgl")]
#[wasm_bindgen_test]
fn webgl_create_renderer_twice() {
    use vello_hybrid::WebGlRenderer;

    let canvas = create_canvas(16, 16);

    let _ = WebGlRenderer::new(&canvas);
    let _ = WebGlRenderer::new(&canvas);
}
