// Copyright 2026 the Vello Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Regenerate the probe reference assets in `vello_common/assets`.

use bytemuck::cast_slice;
use std::{path::PathBuf, sync::LazyLock};

#[cfg(not(target_arch = "wasm32"))]
use oxipng::Options;
use vello_common::{
    kurbo::Rect,
    paint::PaintType,
    pixmap::Pixmap,
    probe::{self, ProbeRenderer},
};
use vello_cpu::{Level, RasterizerSettings, RenderContext, RenderMode, RenderSettings, Resources};

static PROBE_PNG_PATH: LazyLock<PathBuf> = LazyLock::new(|| {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../vello_common/assets/probe.png")
});
static PROBE_RGBA_PATH: LazyLock<PathBuf> = LazyLock::new(|| {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../vello_common/assets/probe.rgba")
});

struct ProbeReferenceData {
    png: Vec<u8>,
    rgba: Vec<u8>,
}

struct CpuProbeContext<'a>(&'a mut RenderContext);

impl ProbeRenderer for CpuProbeContext<'_> {
    fn set_paint(&mut self, paint: PaintType) {
        self.0.set_paint(paint);
    }

    fn fill_rect(&mut self, rect: &Rect) {
        self.0.fill_rect(rect);
    }
}

fn render_probe_pixmap() -> Pixmap {
    let (width, height) = probe::canvas_size();
    let settings = RenderSettings {
        level: Level::fallback(),
        num_threads: 0,
    };
    let mut ctx = RenderContext::new_with(width, height, settings);

    probe::draw_scene(&mut CpuProbeContext(&mut ctx));
    ctx.flush();

    let mut resources = Resources::new();
    let mut pixmap = Pixmap::new(width, height);
    ctx.render_with(
        &mut pixmap,
        &mut resources,
        RasterizerSettings {
            render_mode: RenderMode::OptimizeQuality,
            ..Default::default()
        },
    );
    pixmap
}

fn build_probe_reference_data() -> ProbeReferenceData {
    let pixmap = render_probe_pixmap();
    let rgba = cast_slice(&pixmap.clone().take_unpremultiplied()).to_vec();
    let png = pixmap.into_png().unwrap();
    #[cfg(not(target_arch = "wasm32"))]
    let png = oxipng::optimize_from_memory(&png, &Options::max_compression()).unwrap();
    ProbeReferenceData { png, rgba }
}

fn main() {
    let reference = build_probe_reference_data();
    std::fs::write(&*PROBE_RGBA_PATH, reference.rgba).unwrap();
    std::fs::write(&*PROBE_PNG_PATH, reference.png).unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_reference_is_up_to_date() {
        let reference = build_probe_reference_data();
        let committed_rgba = std::fs::read(&*PROBE_RGBA_PATH).unwrap();
        let committed_png = std::fs::read(&*PROBE_PNG_PATH).unwrap();

        assert_eq!(
            committed_rgba, reference.rgba,
            "probe.rgba is out of date; run `cargo run -p vello_sparse_tests --bin regenerate_probe_reference`",
        );
        assert_eq!(
            committed_png, reference.png,
            "probe.png is out of date; run `cargo run -p vello_sparse_tests --bin regenerate_probe_reference`",
        );
    }
}
