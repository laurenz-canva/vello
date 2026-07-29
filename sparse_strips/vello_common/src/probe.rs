// Copyright 2026 the Vello Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Helpers for performing probing to verify the basic capabilities of the device we are
//! running on.

use crate::color::palette::css;
use crate::kurbo::Rect;
use crate::paint::PaintType;
use crate::pixmap::Pixmap;
use alloc::vec::Vec;

const REFERENCE_RGBA: &[u8] = include_bytes!("../assets/probe.rgba");

const ELEMENTS_PER_ROW: usize = 1;
const ELEMENT_MARGIN: f64 = 1.0;

const RECT_SIZE: f64 = 10.0;
const ELEMENTS: [ProbeElement; 1] = [ProbeElement::SolidRect];
/// Per-channel absolute tolerance used when comparing probe pixels.
const CHANNEL_TOLERANCE: u8 = 3;

/// Result of running the renderer probe.
#[derive(Debug, Clone)]
pub enum Probe<E> {
    /// The probe matched the bundled reference image.
    Success,
    /// The probe did not match the bundled reference image.
    Error(ProbeResult),
    /// Rendering the probe scene produces an error.
    RenderError(E),
}

/// Probe failure output.
#[derive(Debug, Clone)]
pub struct ProbeResult {
    /// The expected probe image.
    pub expected: ProbeImage,
    /// The actual probe image.
    pub actual: ProbeImage,
}

/// A probe image stored as RGBA8 bytes.
#[derive(Debug, Clone)]
pub struct ProbeImage {
    /// Width of the image in pixels.
    pub width: u16,
    /// Height of the image in pixels.
    pub height: u16,
    /// The image data as RGBA8 bytes.
    pub data: Vec<u8>,
}

impl<E> Probe<E> {
    /// Returns `true` when the probe matched the bundled reference image.
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success)
    }

    /// Construct a new probe result by inspecting the provided pixmap and comparing it
    /// against the reference output.
    pub fn from_actual(actual: Pixmap) -> Self {
        let (width, height) = canvas_size();
        let expected = ProbeImage {
            width,
            height,
            data: REFERENCE_RGBA.to_vec(),
        };
        let actual = ProbeImage::from_pixmap(actual);
        let matches_reference = expected.width == actual.width
            && expected.height == actual.height
            && expected.data.len() == actual.data.len()
            && expected
                .data
                .chunks_exact(4)
                .zip(actual.data.chunks_exact(4))
                .all(|(expected, actual)| {
                    pixels_within_tolerance(expected, actual, CHANNEL_TOLERANCE)
                });

        if matches_reference {
            Self::Success
        } else {
            Self::Error(ProbeResult { expected, actual })
        }
    }
}

impl ProbeImage {
    fn from_pixmap(pixmap: Pixmap) -> Self {
        Self {
            width: pixmap.width(),
            height: pixmap.height(),
            data: bytemuck::cast_slice(&pixmap.take_unpremultiplied()).to_vec(),
        }
    }
}

/// API necessary to draw the probe scene.
pub trait ProbeRenderer {
    fn set_paint(&mut self, paint: PaintType);
    fn fill_rect(&mut self, rect: &Rect);
}

#[derive(Clone, Copy, Debug)]
enum ProbeElement {
    SolidRect,
}

#[derive(Clone, Copy, Debug)]
struct GridLayout {
    columns: usize,
    rows: usize,
    cell_width: f64,
    cell_height: f64,
}

impl GridLayout {
    fn from_elements(elements: &[ProbeElement]) -> Self {
        let columns = ELEMENTS_PER_ROW.min(elements.len());
        let rows = elements.len().div_ceil(columns);
        let (cell_width, cell_height) = elements
            .iter()
            .copied()
            .map(ProbeElement::bounds)
            .fold((0.0_f64, 0.0_f64), |(max_w, max_h), (w, h)| {
                (max_w.max(w), max_h.max(h))
            });

        Self {
            columns,
            rows,
            cell_width,
            cell_height,
        }
    }

    fn canvas_size(self) -> (u16, u16) {
        let width = self.columns as f64 * self.cell_width
            + self.columns.saturating_sub(1) as f64 * ELEMENT_MARGIN;
        let height = self.rows as f64 * self.cell_height
            + self.rows.saturating_sub(1) as f64 * ELEMENT_MARGIN;
        (width.ceil() as u16, height.ceil() as u16)
    }

    fn canvas_rect(self) -> Rect {
        let (width, height) = self.canvas_size();
        Rect::new(0.0, 0.0, f64::from(width), f64::from(height))
    }

    fn cell_rect(self, index: usize) -> Rect {
        let column = index % self.columns;
        let row = index / self.columns;
        let x0 = column as f64 * (self.cell_width + ELEMENT_MARGIN);
        let y0 = row as f64 * (self.cell_height + ELEMENT_MARGIN);
        Rect::new(x0, y0, x0 + self.cell_width, y0 + self.cell_height)
    }
}

impl ProbeElement {
    fn bounds(self) -> (f64, f64) {
        (
            RECT_SIZE + ELEMENT_MARGIN * 2.0,
            RECT_SIZE + ELEMENT_MARGIN * 2.0,
        )
    }
}

/// Return the canvas size of the shared probe scene.
pub fn canvas_size() -> (u16, u16) {
    GridLayout::from_elements(&ELEMENTS).canvas_size()
}

/// Draw the full shared probe scene into a rendering context.
pub fn draw_scene<T: ProbeRenderer>(ctx: &mut T) {
    let layout = GridLayout::from_elements(&ELEMENTS);
    ctx.set_paint(css::RED.into());
    ctx.fill_rect(&layout.canvas_rect());

    for (index, element) in ELEMENTS.iter().copied().enumerate() {
        draw_probe_element(ctx, layout.cell_rect(index), element);
    }
}

fn pixels_within_tolerance(expected: &[u8], actual: &[u8], channel_tolerance: u8) -> bool {
    if expected[3] == 0 && actual[3] == 0 {
        return true;
    }

    expected
        .iter()
        .zip(actual)
        .all(|(expected, actual)| expected.abs_diff(*actual) <= channel_tolerance)
}

fn draw_probe_element(ctx: &mut impl ProbeRenderer, cell: Rect, element: ProbeElement) {
    match element {
        ProbeElement::SolidRect => {
            ctx.set_paint(css::RED.into());
            ctx.fill_rect(&centered_rect(cell, RECT_SIZE, RECT_SIZE));
        }
    }
}

fn centered_rect(cell: Rect, width: f64, height: f64) -> Rect {
    let center = cell.center();
    Rect::new(
        center.x - width * 0.5,
        center.y - height * 0.5,
        center.x + width * 0.5,
        center.y + height * 0.5,
    )
}
