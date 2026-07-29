// Copyright 2024 the Vello Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// A WGSL shader for rendering sparse strips.
//
// `StripInstance::paint` field encodes the color source.
// The color source determines where the fragment shader gets color data from.
// More details in the `StripInstance` documentation below.
//
// `StripInstance::payload` field can either encode a color or a layer texture origin.
// - If color source is payload, the fragment shader uses the color directly.
// - If color source is layer, the fragment shader samples the layer texture and applies layer opacity.
// More details in the `StripInstance` documentation below.


// Color source modes - where the fragment shader gets color data from.
// Use a payload color.
const COLOR_SOURCE_PAYLOAD: u32 = 0u;
// Sample from a rendered layer texture.
const COLOR_SOURCE_LAYER: u32 = 1u;

const RECT_STRIP_FLAG: u32 = 0x80000000u;

struct Config {
    // Width of the rendering target
    width: u32,
    // Height of the rendering target
    height: u32,
    // Height of a strip in the rendering
    // CAUTION: When changing this value, you must also update the fragment shader's
    // logic to handle the new strip height.
    strip_height: u32,
    // Reserved to preserve the CPU-side uniform layout.
    _reserved_alpha: u32,
    // Reserved to preserve the CPU-side uniform layout.
    _reserved: u32,
    // An offset to apply to the strip.
    //
    // In most cases, these will be zero. However,
    // when rendering filter layers, we need to account for the 1) shift caused by
    // only rendering the tight bounding box of the filter layer and 2) the offset
    // within the atlas where the filter layer will be rendered to.
    strip_offset_x: i32,
    strip_offset_y: i32,
    // Whether to flip the y-component of the NDC coordinates.
    ndc_y_negate: u32,
}

// A `StripInstance` can represent either a normal strip or a rect strip.
// The two modes are distinguished by RECT_STRIP_FLAG (bit 31 of `paint_and_rect_flag`).
//
// Depending on the active mode, the fields are interpreted as follows:
//
//   Field                 | Normal strip                      | Rect strip
//   ----------------------+-----------------------------------+-----------------------------------
//   xy                    | Strip position                    | Rect top-left (snapped outward)
//   widths_or_rect_height | [width, dense_width]              | [width, height] (both snapped)
//   payload               | Color / scene coords / layer xy   | Color / scene coords / layer xy
//   paint_and_rect_flag   | Paint encoding                    | Paint encoding | RECT_STRIP_FLAG
//
//
// `paint_and_rect_flag` bit layout:
//   - Bit  31:    `RECT_STRIP_FLAG`  0 = normal strip, 1 = rect strip
//   - Bits 29-30: `color_source`     0 = use payload, 1 = use layer texture
//   - Bits 0-28:  Usage depends on color_source:
//     - When color_source = 0 (COLOR_SOURCE_PAYLOAD): unused
//     - When color_source = 1 (COLOR_SOURCE_LAYER):
//       - Bits 0-7: opacity (0-255)
//       - Bits 8-28: unused
//
// Decision tree for paint/payload interpretation:
//
// color_source = 0 (COLOR_SOURCE_PAYLOAD) - Use payload data directly
// └── payload = [r, g, b, a] RGBA (packed as u8s)
//
// color_source = 1 (COLOR_SOURCE_LAYER) - Use rendered layer texture
// ├── payload = [x, y] source layer texture origin (packed as u16s)
// └── bits 0-7 = opacity
struct StripInstance {
    // [x, y] packed as u16's
    // x, y — coordinates of the strip or rect
    @location(0) xy: u32,
    // [width, dense_width_or_rect_height] packed as u16's
    // width — width of the strip or rect
    // For rect strips, the upper 16 bits hold the rectangle height.
    @location(1) widths_or_rect_height: u32,
    // See StripInstance documentation above.
    @location(3) payload: u32,
    // See StripInstance documentation above.
    @location(4) paint_and_rect_flag: u32,
}

struct VertexOutput {
    // Render type for the strip
    @location(0) @interpolate(flat) paint_and_rect_flag: u32,
    // Source coordinates for layer sampling.
    @location(1) sample_xy: vec2<f32>,
    // Packed paint payload or layer sample coordinate.
    @location(2) @interpolate(flat) payload: u32,
    // Normalized device coordinates (NDC) for the current vertex
    @builtin(position) position: vec4<f32>,
};

// TODO: Measure performance of moving to a separate group
@group(0) @binding(1)
var<uniform> config: Config;

@vertex
fn vs_main(
    @builtin(vertex_index) in_vertex_index: u32,
    instance: StripInstance,
) -> VertexOutput {
    var out: VertexOutput;
    out.sample_xy = vec2(0.0);
    // Map vertex_index (0-3) to quad corners:
    // 0 → (0,0), 1 → (1,0), 2 → (0,1), 3 → (1,1)
    let x = f32(in_vertex_index & 1u);
    let y = f32(in_vertex_index >> 1u);
    // Unpack the x and y coordinates from the packed u32 instance.xy
    let x0 = instance.xy & 0xffffu;
    let y0 = instance.xy >> 16u;
    let width = instance.widths_or_rect_height & 0xffffu;
    let dense_width = instance.widths_or_rect_height >> 16u;

    let is_rect = (instance.paint_and_rect_flag & RECT_STRIP_FLAG) != 0u;
    var height = config.strip_height;
    if is_rect {
        height = dense_width;
    }
    // Calculate the pixel coordinates of the current vertex within the strip.
    // Don't forget to apply the strip offset!
    let pix_x = f32(i32(x0) + config.strip_offset_x) + x * f32(width);
    let pix_y = f32(i32(y0) + config.strip_offset_y) + y * f32(height);
    // Convert pixel coordinates to normalized device coordinates (NDC)
    // NDC ranges from -1 to 1, with (0,0) at the center of the viewport
    let ndc_x = pix_x * 2.0 / f32(config.width) - 1.0;
    let ndc_y = 1.0 - pix_y * 2.0 / f32(config.height);

    let color_source = (instance.paint_and_rect_flag >> 29u) & 0x3u;
    if color_source == COLOR_SOURCE_LAYER {
        let src_x = instance.payload & 0xffffu;
        let src_y = instance.payload >> 16u;
        out.sample_xy = vec2<f32>(
            f32(src_x) + x * f32(width),
            f32(src_y) + y * f32(height),
        );
    }

    // Flip it based on the flag.
    let final_ndc_y = select(ndc_y, -ndc_y, config.ndc_y_negate != 0u);
    out.position = vec4<f32>(ndc_x, final_ndc_y, 0.0, 1.0);
    out.payload = instance.payload;
    out.paint_and_rect_flag = instance.paint_and_rect_flag;

    return out;
}

@group(0) @binding(2)
var layer_input_texture: texture_2d<f32>;

@fragment
fn fs_main(
    @location(0) @interpolate(flat) paint_and_rect_flag: u32,
    @location(1) sample_xy: vec2<f32>,
    @location(2) @interpolate(flat) payload: u32,
) -> @location(0) vec4<f32> {
    let color_source = (paint_and_rect_flag >> 29u) & 0x3u;
    var final_color: vec4<f32>;

    if color_source == COLOR_SOURCE_PAYLOAD {
        final_color = unpack4x8unorm(payload);
    } else if color_source == COLOR_SOURCE_LAYER {
        let layer_opacity = f32(paint_and_rect_flag & 0xffu) * (1.0 / 255.0);
        final_color = layer_opacity * textureLoad(layer_input_texture, vec2<i32>(sample_xy), 0);
    } else {
        final_color = vec4<f32>(0.0);
    }

    return final_color;
}
