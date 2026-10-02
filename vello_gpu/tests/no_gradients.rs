// Copyright 2026 the Vello Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Validate the WGPU pipeline and draw bindings when no gradient feature is enabled.

#![cfg(all(
    feature = "wgpu",
    not(target_arch = "wasm32"),
    not(any(
        feature = "gradient_linear",
        feature = "gradient_radial",
        feature = "gradient_sweep"
    ))
))]

use vello_common::kurbo::Rect;
use vello_gpu::{
    ClearSettings, RenderSize, RenderTargetConfig, Renderer, Scene, TargetInit, TextureBindings,
};

#[test]
#[ignore = "requires a GPU adapter; use --no-default-features --features wgpu_default and --ignored"]
fn render_without_gradient_bindings() {
    let instance = wgpu::Instance::default();
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default())).unwrap();
    let config = RenderTargetConfig {
        format: wgpu::TextureFormat::Rgba8Unorm,
        width: 16,
        height: 16,
    };
    let (mut renderer, mut resources) = Renderer::new(&device, &config);
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("No-gradient render target"),
        size: wgpu::Extent3d {
            width: config.width.into(),
            height: config.height.into(),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: config.format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let mut scene = Scene::new(config.width, config.height);
    scene.push_opacity_layer(0.5);
    scene.fill_rect(&Rect::new(1.5, 1.5, 14.5, 14.5));
    scene.pop_layer();
    let mut encoder = device.create_command_encoder(&Default::default());
    renderer
        .render(
            &scene,
            &mut resources,
            &device,
            &queue,
            &mut encoder,
            &RenderSize {
                width: config.width,
                height: config.height,
            },
            &texture.create_view(&Default::default()),
            None,
            &TextureBindings::new(),
            TargetInit::Clear(ClearSettings::default()),
        )
        .unwrap();
    queue.submit([encoder.finish()]);
    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
}
