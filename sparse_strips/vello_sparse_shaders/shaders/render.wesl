// Copyright 2024 the Vello Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

struct Config {
    width: u32,
    height: u32,
    strip_height: u32,
    alphas_tex_width_bits: u32,
    encoded_paints_tex_width_bits: u32,
    strip_offset_x: i32,
    strip_offset_y: i32,
    ndc_y_negate: u32,
}

struct StripInstance {
    @location(0) xy: u32,
    @location(1) widths_or_rect_height: u32,
    @location(4) paint_and_rect_flag: u32,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
}

@group(0) @binding(1)
var<uniform> config: Config;

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    instance: StripInstance,
) -> VertexOutput {
    var out: VertexOutput;
    let corner_x = f32(vertex_index & 1u);
    let corner_y = f32(vertex_index >> 1u);
    let x0 = instance.xy & 0xffffu;
    let y0 = instance.xy >> 16u;
    let width = instance.widths_or_rect_height & 0xffffu;
    let packed_height = instance.widths_or_rect_height >> 16u;
    let is_rect = (instance.paint_and_rect_flag & 0x80000000u) != 0u;
    var height = config.strip_height;
    if is_rect {
        height = packed_height;
    }
    let pixel_x = f32(x0) + corner_x * f32(width);
    let pixel_y = f32(y0) + corner_y * f32(height);
    let ndc_x = pixel_x * 2.0 / f32(config.width) - 1.0;
    let ndc_y = 1.0 - pixel_y * 2.0 / f32(config.height);
    out.position = vec4<f32>(ndc_x, ndc_y, 0.0, 1.0);
    return out;
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return vec4<f32>(1.0, 0.0, 0.0, 1.0);
}
