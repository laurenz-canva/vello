// Copyright 2024 the Vello Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// A WGSL shader for rendering sparse strips with alpha blending.
//
// Each strip instance represents a horizontal slice of the rendered output and consists of:
// 1. A variable-width region of alpha values for semi-transparent rendering
// 2. A solid color region for fully opaque areas
//
// The alpha values are stored in a texture and sampled during fragment shading.
// This approach optimizes memory usage by only storing alpha data where needed.
//
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
    // Number of trailing zeros in alphas_tex_width (log2 of width).
    // Pre-calculated on CPU since WebGL2 doesn't support `firstTrailingBit`.
    alphas_tex_width_bits: u32,
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

// A `StripInstance` can represent either a **normal strip** (representing a sparse fill or alpha fill of height
// Tile::HEIGHT) or a **rect strip** (an entire rectangle rendered as a single quad, with anti-aliasing support).
// The two modes are distinguished by RECT_STRIP_FLAG (bit 31 of `paint_and_rect_flag`).
//
// Depending on the active mode, the fields are interpreted as follows:
//
//   Field                 | Normal strip                      | Rect strip
//   ----------------------+-----------------------------------+-----------------------------------
//   xy                    | Strip position                    | Rect top-left (snapped outward)
//   widths_or_rect_height | [width, dense_width]              | [width, height] (both snapped)
//   col_idx_or_rect_frac  | Alpha column index                | Packed AA edge fractions (4 × u8)
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
    // [width, dense_width] packed as u16's
    // width — width of the strip or rect
    // dense_width — width of the portion where alpha blending should be applied
    // Note that currently, if the strip instance represents an actual strip (i.e. an anti-aliased region),
    // width = dense_width. If the StripInstance represents a sparse fill region, then dense_width = 0.
    // For rect strips, dense_width is repurposed to hold the rectangle height.
    // TODO: In the future, this could be optimized such that `width` always represents the width and a simple
    // 1-bit flag is used to distinguish between sparse fill region and strip. This frees up 15 other bits.
    // Otherwise, it might also be possible to merge a strip and sparse fill command into a single strip instance.
    @location(1) widths_or_rect_height: u32,
    // For normal strips: alpha texture column index where this strip's alpha values begin.
    // There are [`Config::strip_height`] alpha values per column.
    // For rect strips: packed fractional edge offsets for AA.
    @location(2) col_idx_or_rect_frac: u32,
    // See StripInstance documentation above.
    @location(3) payload: u32,
    // See StripInstance documentation above.
    @location(4) paint_and_rect_flag: u32,
    // Painter's-order index for z-depth computation.
    @location(5) depth_index: u32,
}

struct VertexOutput {
    // Render type for the strip
    @location(0) @interpolate(flat) paint_and_rect_flag: u32,
    // Texture coordinates for the current fragment
    @location(1) tex_coord: vec2<f32>,
    // Source coordinates for layer sampling.
    @location(2) sample_xy: vec2<f32>,
    // For normal strips: ending x-position of the dense (alpha) region.
    // For rect strips: packed dimensions (width | height << 16).
    @location(3) @interpolate(flat) dense_end_or_rect_size: u32,
    // Packed paint payload or layer sample coordinate.
    @location(4) @interpolate(flat) payload: u32,
    // Packed fractional edge offsets for rectangles.
    // Bits 0-7: x0, 8-15: y0, 16-23: x1, 24-31: y1.
    // Zero for normal strips.
    @location(5) @interpolate(flat) rect_frac: u32,
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
        out.dense_end_or_rect_size = width | (dense_width << 16u);
        out.rect_frac = instance.col_idx_or_rect_frac;
    } else {
        out.dense_end_or_rect_size = instance.col_idx_or_rect_frac + dense_width;
        out.rect_frac = 0u;
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

    let col_offset = select(f32(instance.col_idx_or_rect_frac), 0.0, is_rect);
    out.tex_coord = vec2<f32>(col_offset + x * f32(width), y * f32(height));

    // Divide by a power of 2 to ensure exact f32 arithmetic (and divide by the expected depth
    // buffer precision of 24 bits).
    let z = 1.0 - f32(instance.depth_index) / f32(1u << 24u);
    // Flip it based on the flag.
    let final_ndc_y = select(ndc_y, -ndc_y, config.ndc_y_negate != 0u);
    out.position = vec4<f32>(ndc_x, final_ndc_y, z, 1.0);
    out.payload = instance.payload;
    out.paint_and_rect_flag = instance.paint_and_rect_flag;

    return out;
}

@group(0) @binding(0)
var alphas_texture: texture_2d<u32>;

@group(0) @binding(2)
var layer_input_texture: texture_2d<f32>;

@fragment
fn fs_main(
    @location(0) @interpolate(flat) paint_and_rect_flag: u32,
    @location(1) tex_coord: vec2<f32>,
    @location(2) sample_xy: vec2<f32>,
    @location(3) @interpolate(flat) dense_end_or_rect_size: u32,
    @location(4) @interpolate(flat) payload: u32,
    @location(5) @interpolate(flat) rect_frac: u32,
) -> @location(0) vec4<f32> {
    var alpha = 1.0;
    let is_rect = (paint_and_rect_flag & RECT_STRIP_FLAG) != 0u;
    // TODO: Explore doing these calculations only for rectangle parts that actually need anti-aliasing. See
    // https://github.com/linebender/vello/pull/1482#discussion_r2861311034
    if is_rect && rect_frac != 0u {
        let frac = unpack4x8unorm(rect_frac);
        // Calculate how much of the pixel is actually covered by the rect.
        // We do this by simply calculating the fractions in the x and y direction, and
        // then multiplying them.
        // For (maybe?) better performance, we calculate the x and y dimension in a single
        // pass by packing everything into a vec2.
        let rect_size = vec2<f32>(f32(dense_end_or_rect_size & 0xFFFFu), f32(dense_end_or_rect_size >> 16u));
        let tc = tex_coord;
        // + 0.5 and -0.5 since the fragment shader positions the coordinates in the center of the pixel.
        let bottom_and_right = min(tc + 0.5, rect_size - frac.zw);
        let top_and_left = max(tc - 0.5, frac.xy);
        let a = clamp(bottom_and_right - top_and_left, vec2(0.0), vec2(1.0));
        alpha = a.x * a.y;
    } else if !is_rect && dense_end_or_rect_size != 0u {
        let x = u32(floor(tex_coord.x));
        let y = u32(floor(tex_coord.y));
        // Retrieve alpha value from the texture. We store 16 1-byte alpha
        // values per texel, with each color channel packing 4 alpha values.
        // The code here assumes the strip height is 4, i.e., each color
        // channel encodes the alpha values for a single column within a strip.
        // Divide x by 4 to get the texel position.
        let alphas_index = x;
        let tex_dimensions = textureDimensions(alphas_texture);
        let alphas_tex_width = tex_dimensions.x;
        // Which texel contains the alpha values for this column
        let texel_index = alphas_index / 4u;
        // Which channel (R,G,B,A) in the texel contains the alpha values for this column
        let channel_index = alphas_index % 4u;
        // Calculate texel coordinates
        let tex_x = texel_index & (alphas_tex_width - 1u);
        let tex_y = texel_index >> config.alphas_tex_width_bits;

        // Load all 4 channels from the texture
        let rgba_values = textureLoad(alphas_texture, vec2<u32>(tex_x, tex_y), 0);

        // Get the column's alphas from the appropriate RGBA channel based on the index
        let alphas_u32 = unpack_alphas_from_channel(rgba_values, channel_index);
        // Extract the alpha value for the current y-position from the packed u32 data
        alpha = f32((alphas_u32 >> (y * 8u)) & 0xffu) * (1.0 / 255.0);
    }
    // Apply the alpha value to the unpacked RGBA color or sampled paint.
    let color_source = (paint_and_rect_flag >> 29u) & 0x3u;
    var final_color: vec4<f32>;

    if color_source == COLOR_SOURCE_PAYLOAD {
        final_color = alpha * unpack4x8unorm(payload);
    } else if color_source == COLOR_SOURCE_LAYER {
        let layer_opacity = f32(paint_and_rect_flag & 0xffu) * (1.0 / 255.0);
        final_color = alpha * layer_opacity * textureLoad(layer_input_texture, vec2<i32>(sample_xy), 0);
    } else {
        final_color = vec4<f32>(0.0);
    }

    return final_color;
}

fn unpack_alphas_from_channel(rgba: vec4<u32>, channel_index: u32) -> u32 {
    switch channel_index {
        case 0u: { return rgba.x; }
        case 1u: { return rgba.y; }
        case 2u: { return rgba.z; }
        case 3u: { return rgba.w; }
        // Fallback, should never happen
        default: { return rgba.x; }
    }
}
