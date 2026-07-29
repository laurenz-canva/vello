// Copyright 2026 the Vello Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! GPU paint packing for scheduled strip draws.

use vello_common::TextureId;
use vello_common::encode::{EncodedKind, EncodedPaint};
use vello_common::paint::Paint;

const COLOR_SOURCE_PAYLOAD: u32 = 0;
pub(crate) const COLOR_SOURCE_LAYER: u32 = 1;

// See the layout information in `render.wesl`.
pub(crate) const COLOR_SOURCE_SHIFT: u32 = 29;

/// Shader-ready paint metadata for a strip.
#[derive(Clone, Copy)]
pub(crate) struct PackedPaint {
    /// Value source for the strip's payload field.
    payload: u32,
    /// Packed paint kind, source, and data offset.
    pub(crate) paint: u32,
    /// External texture required by this paint, if any.
    pub(crate) external_texture_id: Option<TextureId>,
    /// Whether the paint is fully opaque.
    pub(crate) opaque: bool,
}

impl PackedPaint {
    pub(crate) fn payload_at(self, _x: u16, _y: u16) -> u32 {
        self.payload
    }
}

/// Resolves recorded paints to their encoded GPU offsets.
#[derive(Debug, Clone, Copy)]
pub(crate) struct PaintResolver<'a> {
    /// Encoded non-solid paints indexed by [`Paint`].
    encoded: &'a [EncodedPaint],
    /// GPU data offset corresponding to each encoded paint.
    _gpu_offsets: &'a [u32],
}

impl<'a> PaintResolver<'a> {
    pub(crate) fn new(encoded: &'a [EncodedPaint], gpu_offsets: &'a [u32]) -> Self {
        Self {
            encoded,
            _gpu_offsets: gpu_offsets,
        }
    }

    #[inline]
    pub(crate) fn pack(self, paint: &Paint) -> PackedPaint {
        match paint {
            Paint::Solid(color) => PackedPaint {
                payload: color.as_premul_rgba8().to_u32(),
                paint: COLOR_SOURCE_PAYLOAD << COLOR_SOURCE_SHIFT,
                external_texture_id: None,
                opaque: color.is_opaque(),
            },
            Paint::Indexed(indexed_paint) => {
                let paint_id = indexed_paint.index();
                let encoded_paint = &self.encoded[paint_id];

                match encoded_paint {
                    EncodedPaint::Image(_) => {
                        unimplemented!("images are temporarily disabled")
                    }
                    EncodedPaint::ExternalTexture(_) => {
                        unimplemented!("external textures are temporarily disabled")
                    }
                    EncodedPaint::Gradient(gradient) => match &gradient.kind {
                        EncodedKind::Linear(_) => {
                            unimplemented!("linear gradients are temporarily disabled")
                        }
                        EncodedKind::Radial(_) => {
                            unimplemented!("radial gradients are temporarily disabled")
                        }
                        EncodedKind::Sweep(_) => {
                            unimplemented!("sweep gradients are temporarily disabled")
                        }
                    },
                    EncodedPaint::BlurredRoundedRect(_) => {
                        unimplemented!("blurred rounded rectangles are temporarily disabled")
                    }
                }
            }
        }
    }
}
