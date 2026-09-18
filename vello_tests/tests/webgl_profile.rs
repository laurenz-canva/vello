// Copyright 2026 the Vello Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use crate::renderer::{HybridRenderer, Renderer};
use crate::util::get_ctx;
use std::hint::black_box;
use vello_common::color::palette::css::BLUE;
use vello_common::kurbo::Rect;
use vello_cpu::RenderMode;
use wasm_bindgen::JsValue;

const WIDTH: u16 = 256;
const HEIGHT: u16 = 256;

fn now() -> f64 {
    web_sys::js_sys::Date::now()
}

fn log(message: String) {
    web_sys::console::log_1(&JsValue::from_str(&message));
}

fn new_renderer() -> HybridRenderer {
    get_ctx::<HybridRenderer>(
        WIDTH,
        HEIGHT,
        false,
        0,
        "fallback",
        RenderMode::OptimizeSpeed,
    )
}

fn encode_scene(renderer: &mut HybridRenderer) {
    renderer.reset();
    renderer.set_paint(BLUE);
    renderer.fill_rect(&Rect::new(0.0, 0.0, f64::from(WIDTH), f64::from(HEIGHT)));
    renderer.flush();
}

#[wasm_bindgen_test::wasm_bindgen_test]
fn webgl_profile() {
    const FRESH_ITERATIONS: usize = 12;
    const REUSED_ITERATIONS: usize = 64;

    let mut fresh_init_ms = 0.0;
    let mut fresh_scene_ms = 0.0;
    let mut fresh_submit_ms = 0.0;
    let mut fresh_finish_ms = 0.0;
    let mut fresh_readback_ms = 0.0;
    let mut renderer_info = None;

    for _ in 0..FRESH_ITERATIONS {
        let start = now();
        let mut renderer = new_renderer();
        fresh_init_ms += now() - start;

        renderer_info.get_or_insert_with(|| renderer.renderer_info());

        let start = now();
        encode_scene(&mut renderer);
        fresh_scene_ms += now() - start;

        let start = now();
        renderer.render();
        fresh_submit_ms += now() - start;

        let start = now();
        renderer.finish();
        fresh_finish_ms += now() - start;

        let start = now();
        black_box(renderer.snapshot());
        fresh_readback_ms += now() - start;
    }

    log(format!(
        "WEBGL_PROFILE {}",
        renderer_info.unwrap_or_else(|| "renderer=unknown".to_owned())
    ));
    log(format!(
        "WEBGL_PROFILE fresh n={FRESH_ITERATIONS} init_ms={fresh_init_ms:.2} scene_ms={fresh_scene_ms:.2} submit_ms={fresh_submit_ms:.2} finish_ms={fresh_finish_ms:.2} readback_ms={fresh_readback_ms:.2}"
    ));

    let start = now();
    let mut renderer = new_renderer();
    let reused_init_ms = now() - start;
    let mut reused_scene_ms = 0.0;
    let mut reused_submit_ms = 0.0;
    let mut reused_finish_ms = 0.0;
    let mut reused_readback_ms = 0.0;
    let mut reused_png_ms = 0.0;

    for _ in 0..REUSED_ITERATIONS {
        let start = now();
        encode_scene(&mut renderer);
        reused_scene_ms += now() - start;

        let start = now();
        renderer.render();
        reused_submit_ms += now() - start;

        let start = now();
        renderer.finish();
        reused_finish_ms += now() - start;

        let start = now();
        let pixmap = renderer.snapshot();
        reused_readback_ms += now() - start;

        let start = now();
        let encoded = pixmap.into_png().unwrap();
        black_box(image::load_from_memory(&encoded).unwrap().into_rgba8());
        reused_png_ms += now() - start;
    }

    log(format!(
        "WEBGL_PROFILE reused n={REUSED_ITERATIONS} init_ms={reused_init_ms:.2} scene_ms={reused_scene_ms:.2} submit_ms={reused_submit_ms:.2} finish_ms={reused_finish_ms:.2} readback_ms={reused_readback_ms:.2} png_roundtrip_ms={reused_png_ms:.2}"
    ));

    let mut batched_scene_ms = 0.0;
    let mut batched_submit_ms = 0.0;
    for _ in 0..REUSED_ITERATIONS {
        let start = now();
        encode_scene(&mut renderer);
        batched_scene_ms += now() - start;

        let start = now();
        renderer.render();
        batched_submit_ms += now() - start;
    }
    let start = now();
    renderer.finish();
    let batched_finish_ms = now() - start;

    let start = now();
    black_box(renderer.snapshot());
    let batched_readback_ms = now() - start;

    log(format!(
        "WEBGL_PROFILE batched n={REUSED_ITERATIONS} scene_ms={batched_scene_ms:.2} submit_ms={batched_submit_ms:.2} finish_ms={batched_finish_ms:.2} final_readback_ms={batched_readback_ms:.2}"
    ));
}
