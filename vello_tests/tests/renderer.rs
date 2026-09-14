// Copyright 2025 the Vello Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[cfg(all(target_arch = "wasm32", feature = "webgl"))]
use std::cell::RefCell;
#[cfg(not(all(target_arch = "wasm32", feature = "webgl")))]
use std::collections::HashMap;
use std::sync::Arc;
#[cfg(not(all(target_arch = "wasm32", feature = "webgl")))]
use std::sync::{Mutex, MutexGuard};
#[cfg(not(target_arch = "wasm32"))]
use std::sync::OnceLock;

use glifo::GlyphRunBackend;
use vello_common::color::{AlphaColor, Srgb};
use vello_common::filter_effects::Filter;
use vello_common::kurbo::{Affine, BezPath, Rect, Stroke};
use vello_common::mask::Mask;
use vello_common::paint::{ImageId, ImageSource, PaintType, Tint};
use vello_common::peniko::{BlendMode, Fill, FontData};
use vello_common::pixmap::Pixmap;
use vello_cpu::{
    Level, RasterizerSettings, RenderContext, RenderMode, RenderSettings, Resources,
    TargetInit as CpuTargetInit,
};
use vello_gpu::{
    ClearSettings, RectU16, RenderSettings as HybridRenderSettings, Resources as HybridResources,
    Scene, TargetInit as HybridTargetInit, TextureId,
};
#[cfg(all(target_arch = "wasm32", feature = "webgl"))]
use web_sys::WebGl2RenderingContext;

pub(crate) trait Renderer: Sized {
    type GlyphRunBackend<'a>: GlyphRunBackend<'a>
    where
        Self: 'a;

    fn new(
        width: u16,
        height: u16,
        num_threads: u16,
        level: Level,
        render_mode: RenderMode,
    ) -> Self;
    fn new_with_depth_buffer(
        width: u16,
        height: u16,
        num_threads: u16,
        level: Level,
        render_mode: RenderMode,
        use_depth_buffer: bool,
    ) -> Self {
        assert!(
            use_depth_buffer,
            "this test renderer does not support disabling the depth buffer"
        );
        Self::new(width, height, num_threads, level, render_mode)
    }
    fn fill_path(&mut self, path: &BezPath);
    fn stroke_path(&mut self, path: &BezPath);
    fn fill_rect(&mut self, rect: &Rect);
    fn fill_blurred_rounded_rect(&mut self, rect: &Rect, radius: f32, std_dev: f32, invert: bool);
    fn stroke_rect(&mut self, rect: &Rect);
    fn glyph_run(
        &mut self,
        font: &FontData,
    ) -> glifo::GlyphRunBuilder<'_, Self::GlyphRunBackend<'_>>;
    fn push_layer(
        &mut self,
        clip_path: Option<&BezPath>,
        blend_mode: Option<BlendMode>,
        opacity: Option<f32>,
        mask: Option<Mask>,
        filter: Option<Filter>,
    );
    fn flush(&mut self);
    fn push_clip_layer(&mut self, path: &BezPath);
    fn push_clip_path(&mut self, path: &BezPath);
    fn push_blend_layer(&mut self, blend_mode: BlendMode);
    fn push_opacity_layer(&mut self, opacity: f32);
    fn push_mask_layer(&mut self, mask: Mask);
    fn push_filter_layer(&mut self, filter: Filter);
    fn pop_layer(&mut self);
    fn pop_clip_path(&mut self);
    fn set_stroke(&mut self, stroke: Stroke);
    fn set_mask(&mut self, mask: Mask);
    fn set_paint(&mut self, paint: impl Into<PaintType>);
    fn set_tint(&mut self, tint: Option<Tint>);
    fn set_paint_transform(&mut self, affine: Affine);
    fn set_fill_rule(&mut self, fill_rule: Fill);
    fn set_transform(&mut self, transform: Affine);
    fn set_aliasing_threshold(&mut self, aliasing_threshold: Option<u8>);
    fn set_blend_mode(&mut self, blend_mode: BlendMode);
    fn set_filter_effect(&mut self, filter: Filter);
    fn reset_filter_effect(&mut self);
    fn reset(&mut self);
    fn set_target_init(&mut self, target_init: HybridTargetInit<'static>);
    fn render(&mut self);
    fn snapshot(&mut self) -> Pixmap;
    fn register_external_texture(&mut self, pixmap: Arc<Pixmap>) -> TextureId;
    fn get_image_source(&mut self, pixmap: Arc<Pixmap>) -> ImageSource;
    fn register_image(&mut self, pixmap: Arc<Pixmap>) -> ImageId;
}

pub(crate) struct CpuRenderer {
    ctx: RenderContext,
    resources: Resources,
    render_mode: RenderMode,
    target: Pixmap,
    target_init: HybridTargetInit<'static>,
}

impl Renderer for CpuRenderer {
    type GlyphRunBackend<'a> = vello_cpu::CpuGlyphRunBackend<'a>;

    fn new(
        width: u16,
        height: u16,
        num_threads: u16,
        level: Level,
        render_mode: RenderMode,
    ) -> Self {
        let settings = RenderSettings { level, num_threads };
        Self {
            ctx: RenderContext::new_with(width, height, settings),
            resources: Resources::new(),
            render_mode,
            target: Pixmap::new(width, height),
            target_init: HybridTargetInit::Clear(ClearSettings::default()),
        }
    }

    fn fill_path(&mut self, path: &BezPath) {
        self.ctx.fill_path(path);
    }

    fn stroke_path(&mut self, path: &BezPath) {
        self.ctx.stroke_path(path);
    }

    fn fill_rect(&mut self, rect: &Rect) {
        self.ctx.fill_rect(rect);
    }

    fn fill_blurred_rounded_rect(&mut self, rect: &Rect, radius: f32, std_dev: f32, invert: bool) {
        self.ctx
            .fill_blurred_rounded_rect(rect, radius, std_dev, invert);
    }

    fn stroke_rect(&mut self, rect: &Rect) {
        self.ctx.stroke_rect(rect);
    }

    fn glyph_run(
        &mut self,
        font: &FontData,
    ) -> glifo::GlyphRunBuilder<'_, Self::GlyphRunBackend<'_>> {
        self.ctx.glyph_run(&mut self.resources, font)
    }

    fn push_layer(
        &mut self,
        clip_path: Option<&BezPath>,
        blend_mode: Option<BlendMode>,
        opacity: Option<f32>,
        mask: Option<Mask>,
        filter: Option<Filter>,
    ) {
        self.ctx
            .push_layer(clip_path, blend_mode, opacity, mask, filter);
    }

    fn flush(&mut self) {
        self.ctx.flush();
    }

    fn push_clip_layer(&mut self, path: &BezPath) {
        self.ctx.push_clip_layer(path);
    }

    fn push_clip_path(&mut self, path: &BezPath) {
        self.ctx.push_clip_path(path);
    }

    fn push_blend_layer(&mut self, blend_mode: BlendMode) {
        self.ctx.push_blend_layer(blend_mode);
    }

    fn push_opacity_layer(&mut self, opacity: f32) {
        self.ctx.push_opacity_layer(opacity);
    }

    fn push_mask_layer(&mut self, mask: Mask) {
        self.ctx.push_mask_layer(mask);
    }

    fn push_filter_layer(&mut self, filter: Filter) {
        self.ctx.push_filter_layer(filter);
    }

    fn pop_layer(&mut self) {
        self.ctx.pop_layer();
    }

    fn pop_clip_path(&mut self) {
        self.ctx.pop_clip_path();
    }

    fn set_stroke(&mut self, stroke: Stroke) {
        self.ctx.set_stroke(stroke);
    }

    fn set_mask(&mut self, mask: Mask) {
        self.ctx.set_mask(mask);
    }

    fn set_paint(&mut self, paint: impl Into<PaintType>) {
        self.ctx.set_paint(paint);
    }

    fn set_tint(&mut self, tint: Option<Tint>) {
        self.ctx.set_tint(tint);
    }

    fn set_paint_transform(&mut self, affine: Affine) {
        self.ctx.set_paint_transform(affine);
    }

    fn set_fill_rule(&mut self, fill_rule: Fill) {
        self.ctx.set_fill_rule(fill_rule);
    }

    fn set_transform(&mut self, transform: Affine) {
        self.ctx.set_transform(transform);
    }

    fn set_aliasing_threshold(&mut self, aliasing_threshold: Option<u8>) {
        self.ctx.set_aliasing_threshold(aliasing_threshold);
    }

    fn set_blend_mode(&mut self, blend_mode: BlendMode) {
        self.ctx.set_blend_mode(blend_mode);
    }

    fn set_filter_effect(&mut self, filter: Filter) {
        self.ctx.set_filter_effect(filter);
    }

    fn reset_filter_effect(&mut self) {
        self.ctx.reset_filter_effect();
    }

    fn reset(&mut self) {
        self.ctx.reset();
    }

    fn set_target_init(&mut self, target_init: HybridTargetInit<'static>) {
        self.target_init = target_init;
    }

    fn render(&mut self) {
        let target_init = match self.target_init {
            HybridTargetInit::SrcOver => CpuTargetInit::SrcOver,
            HybridTargetInit::Clear(ClearSettings::Viewport { color }) => {
                CpuTargetInit::Clear(color)
            }
            HybridTargetInit::Clear(ClearSettings::Rects { color, rects }) => {
                apply_rect_clear(&mut self.target, color, rects);

                CpuTargetInit::SrcOver
            }
        };

        self.ctx.render_with(
            &mut self.target,
            &mut self.resources,
            RasterizerSettings {
                render_mode: self.render_mode,
                target_init,
                ..Default::default()
            },
        );
    }

    fn snapshot(&mut self) -> Pixmap {
        self.target.clone()
    }

    fn register_external_texture(&mut self, _: Arc<Pixmap>) -> TextureId {
        unimplemented!("external textures are only supported by hybrid renderer tests")
    }

    fn get_image_source(&mut self, pixmap: Arc<Pixmap>) -> ImageSource {
        let id = self.resources.register_image(Arc::clone(&pixmap));
        ImageSource::opaque_id_with_transparency_hint(id, pixmap.may_have_transparency())
    }

    fn register_image(&mut self, pixmap: Arc<Pixmap>) -> ImageId {
        self.resources.register_image(pixmap)
    }
}

#[cfg(not(all(target_arch = "wasm32", feature = "webgl")))]
static WGPU_TEST_MUTEX: Mutex<()> = Mutex::new(());

#[cfg(not(target_arch = "wasm32"))]
static NATIVE_GPU_CONTEXT: OnceLock<NativeGpuContext> = OnceLock::new();

#[cfg(not(all(target_arch = "wasm32", feature = "webgl")))]
struct NativeGpuContext {
    device: wgpu::Device,
    queue: wgpu::Queue,
    renderers: Mutex<Vec<PooledNativeRenderer>>,
}

#[cfg(not(all(target_arch = "wasm32", feature = "webgl")))]
struct PooledNativeRenderer {
    width: u16,
    height: u16,
    use_depth_buffer: bool,
    resources: HybridResources,
    texture: wgpu::Texture,
    texture_view: wgpu::TextureView,
    depth_texture_view: Option<wgpu::TextureView>,
    readback_buffer: wgpu::Buffer,
    renderer: vello_gpu::Renderer,
}

#[cfg(not(all(target_arch = "wasm32", feature = "webgl")))]
fn new_gpu_context() -> NativeGpuContext {
    let instance = wgpu::Instance::default();
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::default(),
        force_fallback_adapter: false,
        compatible_surface: None,
    }))
    .expect("Failed to find an appropriate adapter");
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("Vello test device"),
        required_features: wgpu::Features::empty(),
        ..Default::default()
    }))
    .expect("Failed to create device");

    NativeGpuContext {
        device,
        queue,
        renderers: Mutex::new(Vec::new()),
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn native_gpu_context() -> &'static NativeGpuContext {
    NATIVE_GPU_CONTEXT.get_or_init(new_gpu_context)
}

#[cfg(not(target_arch = "wasm32"))]
type TestGpuContext = &'static NativeGpuContext;

#[cfg(all(target_arch = "wasm32", not(feature = "webgl")))]
type TestGpuContext = NativeGpuContext;

#[cfg(not(all(target_arch = "wasm32", feature = "webgl")))]
pub(crate) struct HybridRenderer {
    scene: Scene,
    gpu: TestGpuContext,
    pooled: Option<PooledNativeRenderer>,
    external_textures: HashMap<TextureId, wgpu::TextureView>,
    next_external_texture_id: u64,
    target_init: HybridTargetInit<'static>,
    _gpu_test_guard: Option<MutexGuard<'static, ()>>,
}

#[cfg(not(all(target_arch = "wasm32", feature = "webgl")))]
impl HybridRenderer {
    fn new_with_settings(
        width: u16,
        height: u16,
        settings: HybridRenderSettings,
        use_depth_buffer: bool,
    ) -> Self {
        // Libtest keeps the generated suite in one process. Serialising only GPU-backed test
        // execution lets CPU cases remain parallel while making the expensive device, renderer,
        // resource caches, and render targets reusable between GPU cases.
        let gpu_test_guard = WGPU_TEST_MUTEX
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        #[cfg(not(target_arch = "wasm32"))]
        let gpu = native_gpu_context();
        #[cfg(target_arch = "wasm32")]
        let gpu = new_gpu_context();
        let scene = Scene::new_with(width, height, settings.level);
        let pooled = {
            let mut renderers = gpu.renderers.lock().unwrap();
            if let Some(index) = renderers.iter().position(|renderer| {
                renderer.width == width
                    && renderer.height == height
                    && renderer.use_depth_buffer == use_depth_buffer
            }) {
                renderers.swap_remove(index)
            } else {
                drop(renderers);
                let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
                    label: Some("Render Target"),
                    size: wgpu::Extent3d {
                        width: width.into(),
                        height: height.into(),
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                        | wgpu::TextureUsages::COPY_SRC,
                    view_formats: &[],
                });
                let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
                let (renderer, resources) = vello_gpu::Renderer::new_with(
                    &gpu.device,
                    &vello_gpu::RenderTargetConfig {
                        format: texture.format(),
                        width,
                        height,
                    },
                    settings,
                );
                let render_size = vello_gpu::RenderSize { width, height };
                let depth_texture_view = use_depth_buffer.then(|| {
                    vello_gpu::Renderer::create_depth_texture_view(&gpu.device, &render_size)
                });
                let bytes_per_row = (u32::from(width) * 4).next_multiple_of(256);
                let readback_buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("Vello test readback buffer"),
                    size: u64::from(bytes_per_row) * u64::from(height),
                    usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                    mapped_at_creation: false,
                });

                PooledNativeRenderer {
                    width,
                    height,
                    use_depth_buffer,
                    resources,
                    texture,
                    texture_view,
                    depth_texture_view,
                    readback_buffer,
                    renderer,
                }
            }
        };

        Self {
            scene,
            gpu,
            pooled: Some(pooled),
            external_textures: HashMap::new(),
            next_external_texture_id: 1,
            target_init: HybridTargetInit::Clear(ClearSettings::default()),
            _gpu_test_guard: Some(gpu_test_guard),
        }
    }

    fn return_to_pool(&mut self) {
        if let Some(pooled) = self.pooled.take() {
            self.gpu.renderers.lock().unwrap().push(pooled);
        }
        self._gpu_test_guard = None;
    }

    fn upload_image_with_resources(
        &mut self,
        pixmap: &Arc<Pixmap>,
        label: &'static str,
    ) -> ImageId {
        let mut encoder = self
            .gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some(label) });
        let pooled = self.pooled.as_mut().unwrap();
        let image_id = pooled.renderer.upload_image(
            &mut pooled.resources,
            &self.gpu.device,
            &self.gpu.queue,
            &mut encoder,
            pixmap,
        );
        self.gpu.queue.submit([encoder.finish()]);
        image_id
    }
}

#[cfg(not(all(target_arch = "wasm32", feature = "webgl")))]
impl Drop for HybridRenderer {
    fn drop(&mut self) {
        self.return_to_pool();
    }
}

#[cfg(not(all(target_arch = "wasm32", feature = "webgl")))]
impl Renderer for HybridRenderer {
    type GlyphRunBackend<'a> = vello_gpu::HybridGlyphRunBackend<'a>;

    fn new(width: u16, height: u16, num_threads: u16, level: Level, _: RenderMode) -> Self {
        Self::new_with_depth_buffer(
            width,
            height,
            num_threads,
            level,
            RenderMode::OptimizeSpeed,
            true,
        )
    }

    fn new_with_depth_buffer(
        width: u16,
        height: u16,
        num_threads: u16,
        level: Level,
        _: RenderMode,
        use_depth_buffer: bool,
    ) -> Self {
        if num_threads != 0 {
            panic!("hybrid renderer doesn't support multi-threading");
        }
        if !level.is_fallback() {
            panic!("hybrid renderer doesn't support SIMD");
        }
        let mut settings = HybridRenderSettings::default();
        // Most of the tests are 100x100 by default, and we want to make sure that some visual
        // tests have the chance to cover more complex parts of the Vello GPU scheduler
        // (for example situations where we need to spill to a new page, etc.). Therefore,
        // we make the minimum size smaller than the default.
        settings.memory_settings.layers_config.min_texture_size = vello_gpu::SizeU16::new(100);
        Self::new_with_settings(width, height, settings, use_depth_buffer)
    }

    fn fill_path(&mut self, path: &BezPath) {
        self.scene.fill_path(path);
    }

    fn stroke_path(&mut self, path: &BezPath) {
        self.scene.stroke_path(path);
    }

    fn fill_rect(&mut self, rect: &Rect) {
        self.scene.fill_rect(rect);
    }

    fn fill_blurred_rounded_rect(&mut self, rect: &Rect, radius: f32, std_dev: f32, invert: bool) {
        self.scene
            .fill_blurred_rounded_rect(rect, radius, std_dev, invert);
    }

    fn stroke_rect(&mut self, rect: &Rect) {
        self.scene.stroke_rect(rect);
    }

    fn glyph_run(
        &mut self,
        font: &FontData,
    ) -> glifo::GlyphRunBuilder<'_, Self::GlyphRunBackend<'_>> {
        let pooled = self.pooled.as_mut().unwrap();
        self.scene.glyph_run(&mut pooled.resources, font)
    }

    fn push_layer(
        &mut self,
        clip: Option<&BezPath>,
        blend_mode: Option<BlendMode>,
        opacity: Option<f32>,
        mask: Option<Mask>,
        filter: Option<Filter>,
    ) {
        self.scene
            .push_layer(clip, blend_mode, opacity, mask, filter);
    }

    fn flush(&mut self) {}

    fn push_clip_layer(&mut self, path: &BezPath) {
        self.scene.push_clip_layer(path);
    }

    fn push_clip_path(&mut self, path: &BezPath) {
        self.scene.push_clip_path(path);
    }

    fn push_blend_layer(&mut self, blend_mode: BlendMode) {
        self.scene
            .push_layer(None, Some(blend_mode), None, None, None);
    }

    fn push_opacity_layer(&mut self, opacity: f32) {
        self.scene.push_layer(None, None, Some(opacity), None, None);
    }

    fn push_mask_layer(&mut self, mask: Mask) {
        self.scene.push_mask_layer(mask);
    }

    fn push_filter_layer(&mut self, filter: Filter) {
        self.scene.push_filter_layer(filter);
    }

    fn pop_layer(&mut self) {
        self.scene.pop_layer();
    }

    fn pop_clip_path(&mut self) {
        self.scene.pop_clip_path();
    }

    fn set_stroke(&mut self, stroke: Stroke) {
        self.scene.set_stroke(stroke);
    }

    fn set_mask(&mut self, _: Mask) {
        unimplemented!()
    }

    fn set_paint(&mut self, paint: impl Into<PaintType>) {
        self.scene.set_paint(paint);
    }

    fn set_tint(&mut self, tint: Option<Tint>) {
        self.scene.set_tint(tint);
    }

    fn set_paint_transform(&mut self, affine: Affine) {
        self.scene.set_paint_transform(affine);
    }

    fn set_fill_rule(&mut self, fill_rule: Fill) {
        self.scene.set_fill_rule(fill_rule);
    }

    fn set_transform(&mut self, transform: Affine) {
        self.scene.set_transform(transform);
    }

    fn set_blend_mode(&mut self, blend_mode: BlendMode) {
        self.scene.set_blend_mode(blend_mode);
    }

    fn set_aliasing_threshold(&mut self, aliasing_threshold: Option<u8>) {
        self.scene.set_aliasing_threshold(aliasing_threshold);
    }

    fn set_filter_effect(&mut self, filter: Filter) {
        self.scene.set_filter_effect(filter);
    }

    fn reset_filter_effect(&mut self) {
        self.scene.reset_filter_effect();
    }

    fn reset(&mut self) {
        self.scene.reset();
    }

    fn set_target_init(&mut self, target_init: HybridTargetInit<'static>) {
        self.target_init = target_init;
    }

    fn render(&mut self) {
        let width = self.scene.width();
        let height = self.scene.height();

        let render_size = vello_gpu::RenderSize { width, height };

        let mut texture_bindings = vello_gpu::TextureBindings::new();
        for (texture_id, texture) in &self.external_textures {
            texture_bindings.insert(*texture_id, texture.clone());
        }
        let mut encoder = self
            .gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Vello Render"),
            });
        let pooled = self.pooled.as_mut().unwrap();
        pooled
            .renderer
            .render(
                &self.scene,
                &mut pooled.resources,
                &self.gpu.device,
                &self.gpu.queue,
                &mut encoder,
                &render_size,
                &pooled.texture_view,
                pooled.depth_texture_view.as_ref(),
                &texture_bindings,
                self.target_init,
            )
            .unwrap();

        self.gpu.queue.submit([encoder.finish()]);
    }

    // This method creates device resources every time it is called. This does not matter much for
    // testing, but should not be used as a basis for implementing something real. This would be a
    // very bad example for that.
    fn snapshot(&mut self) -> Pixmap {
        let width = self.scene.width();
        let height = self.scene.height();

        let mut encoder = self
            .gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Vello Readback"),
            });

        let bytes_per_row = (u32::from(width) * 4).next_multiple_of(256);
        let texture_copy_buffer = &self.pooled.as_ref().unwrap().readback_buffer;

        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &self.pooled.as_ref().unwrap().texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: texture_copy_buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_row),
                    rows_per_image: None,
                },
            },
            wgpu::Extent3d {
                width: width.into(),
                height: height.into(),
                depth_or_array_layers: 1,
            },
        );

        self.gpu.queue.submit([encoder.finish()]);

        // Map the buffer for reading
        texture_copy_buffer
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                if result.is_err() {
                    panic!("Failed to map texture for reading");
                }
            });
        self.gpu
            .device
            .poll(wgpu::PollType::wait_indefinitely())
            .unwrap();

        // Read back the pixel data
        let mut pixmap = Pixmap::new(width, height);
        for (row, buf) in texture_copy_buffer
            .slice(..)
            .get_mapped_range()
            .chunks_exact(bytes_per_row as usize)
            .zip(
                pixmap
                    .data_as_u8_slice_mut()
                    .chunks_exact_mut(width as usize * 4),
            )
        {
            buf.copy_from_slice(&row[0..width as usize * 4]);
        }
        texture_copy_buffer.unmap();
        self.return_to_pool();
        pixmap
    }

    fn register_external_texture(&mut self, pixmap: Arc<Pixmap>) -> TextureId {
        let texture_id = TextureId(self.next_external_texture_id);
        self.next_external_texture_id += 1;

        let width = u32::from(pixmap.width());
        let height = u32::from(pixmap.height());
        let texture = self.gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Test External Texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        self.gpu.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            pixmap.data_as_u8_slice(),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width * 4),
                rows_per_image: None,
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        self.external_textures.insert(texture_id, view);
        texture_id
    }

    fn get_image_source(&mut self, pixmap: Arc<Pixmap>) -> ImageSource {
        let image_id = self.upload_image_with_resources(&pixmap, "Upload Test Image");
        ImageSource::opaque_id_with_transparency_hint(image_id, pixmap.may_have_transparency())
    }

    fn register_image(&mut self, pixmap: Arc<Pixmap>) -> ImageId {
        self.upload_image_with_resources(&pixmap, "Register Test Image")
    }
}

#[cfg(all(target_arch = "wasm32", feature = "webgl"))]
struct PooledWebGlRenderer {
    width: u16,
    height: u16,
    use_depth_buffer: bool,
    resources: HybridResources,
    renderer: vello_gpu::WebGlRenderer,
    gl: WebGl2RenderingContext,
}

#[cfg(all(target_arch = "wasm32", feature = "webgl"))]
thread_local! {
    static WEBGL_RENDERER_POOL: RefCell<Vec<PooledWebGlRenderer>> =
        const { RefCell::new(Vec::new()) };
}

#[cfg(all(target_arch = "wasm32", feature = "webgl"))]
pub(crate) struct HybridRenderer {
    scene: Scene,
    pooled: Option<PooledWebGlRenderer>,
    external_textures: vello_gpu::WebGlTextureBindings,
    next_external_texture_id: u64,
    clear_color: AlphaColor<Srgb>,
}

#[cfg(all(target_arch = "wasm32", feature = "webgl"))]
impl HybridRenderer {
    fn upload_image(&mut self, pixmap: &Arc<Pixmap>) -> ImageId {
        let pooled = self.pooled.as_mut().unwrap();
        pooled
            .renderer
            .upload_image(&mut pooled.resources, pixmap)
            .unwrap()
    }

    fn return_to_pool(&mut self) {
        if let Some(pooled) = self.pooled.take() {
            WEBGL_RENDERER_POOL.with(|renderers| renderers.borrow_mut().push(pooled));
        }
    }
}

#[cfg(all(target_arch = "wasm32", feature = "webgl"))]
impl Drop for HybridRenderer {
    fn drop(&mut self) {
        self.return_to_pool();
    }
}

#[cfg(all(target_arch = "wasm32", feature = "webgl"))]
impl Renderer for HybridRenderer {
    type GlyphRunBackend<'a> = vello_gpu::HybridGlyphRunBackend<'a>;

    fn new(width: u16, height: u16, num_threads: u16, level: Level, _: RenderMode) -> Self {
        Self::new_with_depth_buffer(
            width,
            height,
            num_threads,
            level,
            RenderMode::OptimizeSpeed,
            true,
        )
    }

    fn new_with_depth_buffer(
        width: u16,
        height: u16,
        num_threads: u16,
        level: Level,
        _: RenderMode,
        use_depth_buffer: bool,
    ) -> Self {
        use wasm_bindgen::JsCast;
        use web_sys::HtmlCanvasElement;

        if num_threads != 0 {
            panic!("hybrid renderer doesn't support multi-threading");
        }

        if !level.is_fallback() {
            panic!("hybrid renderer doesn't support SIMD");
        }

        let mut settings = HybridRenderSettings::default();
        // See the comment above for why we change the `min_texture_size`.
        settings.memory_settings.layers_config.min_texture_size = vello_gpu::SizeU16::new(100);
        let scene = Scene::new_with(width, height, settings.level);
        let pooled = WEBGL_RENDERER_POOL
            .with(|renderers| {
                let mut renderers = renderers.borrow_mut();
                renderers
                    .iter()
                    .position(|renderer| {
                        renderer.width == width
                            && renderer.height == height
                            && renderer.use_depth_buffer == use_depth_buffer
                    })
                    .map(|index| renderers.swap_remove(index))
            })
            .unwrap_or_else(|| {
                // Create an offscreen HTMLCanvasElement, render the test image to it, and finally
                // read the pixmap back for diff checking.
                let document = web_sys::window().unwrap().document().unwrap();
                let canvas = document
                    .create_element("canvas")
                    .unwrap()
                    .dyn_into::<HtmlCanvasElement>()
                    .unwrap();
                canvas.set_width(width.into());
                canvas.set_height(height.into());
                let (renderer, resources) = vello_gpu::WebGlRenderer::new_with(
                    &canvas,
                    settings,
                    use_depth_buffer,
                )
                .unwrap();
                let gl = canvas
                    .get_context("webgl2")
                    .unwrap()
                    .unwrap()
                    .dyn_into::<WebGl2RenderingContext>()
                    .unwrap();
                PooledWebGlRenderer {
                    width,
                    height,
                    use_depth_buffer,
                    resources,
                    renderer,
                    gl,
                }
            });
        Self {
            scene,
            pooled: Some(pooled),
            external_textures: vello_gpu::WebGlTextureBindings::new(),
            next_external_texture_id: 1,
            clear_color: AlphaColor::TRANSPARENT,
        }
    }

    fn fill_path(&mut self, path: &BezPath) {
        self.scene.fill_path(path);
    }

    fn set_blend_mode(&mut self, blend_mode: BlendMode) {
        self.scene.set_blend_mode(blend_mode);
    }

    fn stroke_path(&mut self, path: &BezPath) {
        self.scene.stroke_path(path);
    }

    fn fill_rect(&mut self, rect: &Rect) {
        self.scene.fill_rect(rect);
    }

    fn fill_blurred_rounded_rect(&mut self, rect: &Rect, radius: f32, std_dev: f32, invert: bool) {
        self.scene
            .fill_blurred_rounded_rect(rect, radius, std_dev, invert);
    }

    fn stroke_rect(&mut self, rect: &Rect) {
        self.scene.stroke_rect(rect);
    }

    fn glyph_run(
        &mut self,
        font: &FontData,
    ) -> glifo::GlyphRunBuilder<'_, Self::GlyphRunBackend<'_>> {
        let pooled = self.pooled.as_mut().unwrap();
        self.scene.glyph_run(&mut pooled.resources, font)
    }

    fn push_clip_path(&mut self, path: &BezPath) {
        self.scene.push_clip_path(path);
    }

    fn push_layer(
        &mut self,
        clip: Option<&BezPath>,
        blend_mode: Option<BlendMode>,
        opacity: Option<f32>,
        mask: Option<Mask>,
        filter: Option<Filter>,
    ) {
        self.scene
            .push_layer(clip, blend_mode, opacity, mask, filter);
    }

    fn flush(&mut self) {}

    fn push_clip_layer(&mut self, path: &BezPath) {
        self.scene.push_clip_layer(path);
    }

    fn push_blend_layer(&mut self, mode: BlendMode) {
        self.scene.push_layer(None, Some(mode), None, None, None);
    }

    fn push_opacity_layer(&mut self, opacity: f32) {
        self.scene.push_layer(None, None, Some(opacity), None, None);
    }

    fn push_mask_layer(&mut self, _: Mask) {
        unimplemented!()
    }

    fn push_filter_layer(&mut self, filter: Filter) {
        self.scene.push_filter_layer(filter);
    }

    fn pop_layer(&mut self) {
        self.scene.pop_layer();
    }

    fn pop_clip_path(&mut self) {
        self.scene.pop_clip_path();
    }

    fn set_stroke(&mut self, stroke: Stroke) {
        self.scene.set_stroke(stroke);
    }

    fn set_mask(&mut self, _: Mask) {
        unimplemented!()
    }

    fn set_paint(&mut self, paint: impl Into<PaintType>) {
        self.scene.set_paint(paint);
    }

    fn set_tint(&mut self, tint: Option<Tint>) {
        self.scene.set_tint(tint);
    }

    fn set_paint_transform(&mut self, affine: Affine) {
        self.scene.set_paint_transform(affine);
    }

    fn set_fill_rule(&mut self, fill_rule: Fill) {
        self.scene.set_fill_rule(fill_rule);
    }

    fn set_transform(&mut self, transform: Affine) {
        self.scene.set_transform(transform);
    }

    fn set_aliasing_threshold(&mut self, aliasing_threshold: Option<u8>) {
        self.scene.set_aliasing_threshold(aliasing_threshold);
    }

    fn set_filter_effect(&mut self, filter: Filter) {
        self.scene.set_filter_effect(filter);
    }

    fn reset_filter_effect(&mut self) {
        self.scene.reset_filter_effect();
    }

    fn reset(&mut self) {
        self.scene.reset();
    }

    fn set_target_init(&mut self, target_init: HybridTargetInit<'static>) {
        self.clear_color = match target_init {
            HybridTargetInit::Clear(ClearSettings::Viewport { color }) => color,
            HybridTargetInit::SrcOver | HybridTargetInit::Clear(ClearSettings::Rects { .. }) => {
                panic!("WebGL only supports clearing the complete viewport")
            }
        };
    }

    fn render(&mut self) {
        let width = self.scene.width();
        let height = self.scene.height();

        let render_size = vello_gpu::RenderSize { width, height };
        let pooled = self.pooled.as_mut().unwrap();
        pooled
            .renderer
            .render(
                &self.scene,
                &mut pooled.resources,
                &render_size,
                &self.external_textures,
                self.clear_color,
            )
            .unwrap();
    }

    fn snapshot(&mut self) -> Pixmap {
        use vello_common::peniko::ImageAlphaType;
        use vello_common::pixmap::PixelMetadata;
        use web_sys::WebGl2RenderingContext;

        let width = self.scene.width();
        let height = self.scene.height();
        let mut pixels = vec![0_u8; (width as usize) * (height as usize) * 4];
        self.pooled
            .as_ref()
            .unwrap()
            .gl
            .read_pixels_with_opt_u8_array(
                0,
                0,
                width.into(),
                height.into(),
                WebGl2RenderingContext::RGBA,
                WebGl2RenderingContext::UNSIGNED_BYTE,
                Some(&mut pixels),
            )
            .unwrap();

        // WebGL framebuffers are y-up, so we need to invert the rows.
        let row_bytes = usize::from(width) * 4;
        let height_usize = usize::from(height);
        for y in 0..height_usize / 2 {
            let (a, b) = pixels.split_at_mut((height_usize - 1 - y) * row_bytes);
            a[y * row_bytes..(y + 1) * row_bytes].swap_with_slice(&mut b[..row_bytes]);
        }

        let pixmap = Pixmap::from_parts(
            pixels,
            width,
            height,
            PixelMetadata::new(ImageAlphaType::AlphaPremultiplied, true),
        );
        self.return_to_pool();
        pixmap
    }

    fn register_external_texture(&mut self, pixmap: Arc<Pixmap>) -> TextureId {
        let texture_id = TextureId(self.next_external_texture_id);
        self.next_external_texture_id += 1;

        let gl = &self.pooled.as_ref().unwrap().gl;
        let texture = gl.create_texture().unwrap();
        gl
            .bind_texture(WebGl2RenderingContext::TEXTURE_2D, Some(&texture));
        gl
            .tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_u8_array(
                WebGl2RenderingContext::TEXTURE_2D,
                0,
                WebGl2RenderingContext::RGBA8 as i32,
                pixmap.width().into(),
                pixmap.height().into(),
                0,
                WebGl2RenderingContext::RGBA,
                WebGl2RenderingContext::UNSIGNED_BYTE,
                Some(pixmap.data_as_u8_slice()),
            )
            .unwrap();
        // `texelFetch` requires a complete texture, which a texture without mipmaps only is once
        // its minification filter no longer samples mipmaps.
        for (param, value) in [
            (
                WebGl2RenderingContext::TEXTURE_MIN_FILTER,
                WebGl2RenderingContext::NEAREST,
            ),
            (
                WebGl2RenderingContext::TEXTURE_MAG_FILTER,
                WebGl2RenderingContext::NEAREST,
            ),
            (
                WebGl2RenderingContext::TEXTURE_WRAP_S,
                WebGl2RenderingContext::CLAMP_TO_EDGE,
            ),
            (
                WebGl2RenderingContext::TEXTURE_WRAP_T,
                WebGl2RenderingContext::CLAMP_TO_EDGE,
            ),
        ] {
            gl.tex_parameteri(WebGl2RenderingContext::TEXTURE_2D, param, value as i32);
        }
        gl.bind_texture(WebGl2RenderingContext::TEXTURE_2D, None);

        self.external_textures.insert(texture_id, texture);
        texture_id
    }

    fn get_image_source(&mut self, pixmap: Arc<Pixmap>) -> ImageSource {
        let image_id = self.upload_image(&pixmap);
        ImageSource::opaque_id_with_transparency_hint(image_id, pixmap.may_have_transparency())
    }

    fn register_image(&mut self, pixmap: Arc<Pixmap>) -> ImageId {
        self.upload_image(&pixmap)
    }
}

// Vello CPU does not expose rectangle clears (because it would be pretty inefficient),
// so we simulate them here for the snapshot tests.
fn apply_rect_clear(pixmap: &mut Pixmap, color: AlphaColor<Srgb>, rects: &[RectU16]) {
    let color = color.premultiply().to_rgba8();
    let width = usize::from(pixmap.width());
    let bounds = RectU16::new(0, 0, pixmap.width(), pixmap.height());
    let data = pixmap.data_mut();

    for rect in rects
        .iter()
        .map(|rect| rect.intersect(bounds))
        .filter(|rect| !rect.is_empty())
    {
        for y in usize::from(rect.y0)..usize::from(rect.y1) {
            let start = y * width + usize::from(rect.x0);
            let end = y * width + usize::from(rect.x1);
            data[start..end].fill(color);
        }
    }
}
