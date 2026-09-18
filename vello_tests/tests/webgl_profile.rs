// Copyright 2026 the Vello Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use crate::renderer::{HybridRenderer, Renderer};
use crate::util::get_ctx;
use std::hint::black_box;
use vello_common::color::palette::css::{BLUE, GREEN, REBECCA_PURPLE, RED};
use vello_common::kurbo::{Circle, Rect, Shape, Stroke};
use vello_cpu::RenderMode;

fn now() -> f64 {
    web_sys::js_sys::Date::now()
}

fn new_renderer(width: u16, height: u16) -> HybridRenderer {
    get_ctx::<HybridRenderer>(
        width,
        height,
        false,
        0,
        "fallback",
        RenderMode::OptimizeSpeed,
    )
}

fn simple_rect(renderer: &mut HybridRenderer) {
    renderer.set_paint(BLUE);
    renderer.fill_rect(&Rect::new(0.0, 0.0, 256.0, 256.0));
}

fn many_paths(renderer: &mut HybridRenderer) {
    let colors = [BLUE, GREEN, RED, REBECCA_PURPLE];
    for y in 0_u16..16 {
        for x in 0_u16..16 {
            renderer.set_paint(
                colors[(usize::from(x) + usize::from(y)) % colors.len()].with_alpha(0.7),
            );
            renderer.fill_path(
                &Circle::new((f64::from(x) * 16.0 + 8.0, f64::from(y) * 16.0 + 8.0), 7.5)
                    .to_path(0.1),
            );
        }
    }
}

fn clipped_layers(renderer: &mut HybridRenderer) {
    for inset in 0..24 {
        let inset = f64::from(inset) * 4.0;
        renderer.push_clip_rect(&Rect::new(inset, inset, 256.0 - inset, 256.0 - inset));
        renderer.push_opacity_layer(0.85);
        renderer.set_paint(if inset as u32 % 8 == 0 { BLUE } else { RED });
        renderer.fill_path(&Circle::new((128.0, 128.0), 128.0 - inset).to_path(0.1));
        renderer.pop_layer();
        renderer.pop_layer();
    }
}

fn large_paths(renderer: &mut HybridRenderer) {
    renderer.set_stroke(Stroke::new(4.0));
    for y in 0..20 {
        for x in 0..20 {
            let center = (f64::from(x) * 48.0 + 24.0, f64::from(y) * 45.0 + 24.0);
            let circle = Circle::new(center, 20.0 + f64::from((x + y) % 4));
            renderer.set_paint(if (x + y) % 2 == 0 { BLUE } else { RED });
            renderer.stroke_path(&circle.to_path(0.1));
        }
    }
}

fn measure_workload(
    name: &str,
    width: u16,
    height: u16,
    iterations: usize,
    mut encode: impl FnMut(&mut HybridRenderer),
) {
    let start = now();
    let mut renderer = new_renderer(width, height);
    let init_ms = now() - start;
    let mut scene_ms = 0.0;
    let mut submit_ms = 0.0;
    let mut finish_ms = 0.0;
    let mut readback_ms = 0.0;
    let mut png_roundtrip_ms = 0.0;

    for _ in 0..iterations {
        let start = now();
        renderer.reset();
        encode(&mut renderer);
        renderer.flush();
        scene_ms += now() - start;

        let start = now();
        renderer.render();
        submit_ms += now() - start;

        let start = now();
        renderer.finish();
        finish_ms += now() - start;

        let start = now();
        let pixmap = renderer.snapshot();
        readback_ms += now() - start;

        let start = now();
        let encoded = pixmap.into_png().unwrap();
        black_box(image::load_from_memory(&encoded).unwrap().into_rgba8());
        png_roundtrip_ms += now() - start;
    }

    wasm_bindgen_test::console_log!(
        "WEBGL_PROFILE workload={name} size={width}x{height} n={iterations} init_ms={init_ms:.2} scene_ms={scene_ms:.2} submit_ms={submit_ms:.2} finish_ms={finish_ms:.2} readback_ms={readback_ms:.2} png_roundtrip_ms={png_roundtrip_ms:.2}"
    );
}

#[wasm_bindgen_test::wasm_bindgen_test]
fn webgl_profile() {
    const FRESH_ITERATIONS: usize = 12;

    let mut fresh_init_ms = 0.0;
    let mut fresh_scene_ms = 0.0;
    let mut fresh_submit_ms = 0.0;
    let mut fresh_finish_ms = 0.0;
    let mut fresh_readback_ms = 0.0;
    let mut renderer_info = None;

    for _ in 0..FRESH_ITERATIONS {
        let start = now();
        let mut renderer = new_renderer(256, 256);
        fresh_init_ms += now() - start;

        renderer_info.get_or_insert_with(|| renderer.renderer_info());

        let start = now();
        renderer.reset();
        simple_rect(&mut renderer);
        renderer.flush();
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

    wasm_bindgen_test::console_log!(
        "WEBGL_PROFILE {}",
        renderer_info.unwrap_or_else(|| "renderer=unknown".to_owned())
    );
    wasm_bindgen_test::console_log!(
        "WEBGL_PROFILE fresh workload=simple_rect size=256x256 n={FRESH_ITERATIONS} init_ms={fresh_init_ms:.2} scene_ms={fresh_scene_ms:.2} submit_ms={fresh_submit_ms:.2} finish_ms={fresh_finish_ms:.2} readback_ms={fresh_readback_ms:.2}"
    );

    measure_workload("simple_rect", 256, 256, 32, simple_rect);
    measure_workload("many_paths", 256, 256, 16, many_paths);
    measure_workload("clipped_layers", 256, 256, 16, clipped_layers);
    measure_workload("large_paths", 1000, 950, 4, large_paths);
}
