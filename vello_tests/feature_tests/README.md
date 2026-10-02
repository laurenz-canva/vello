# GPU shader feature combinations

This crate draws `vello_common::probe::draw_scene` through the regular WGPU and WebGL rendering
APIs. It compares complete rendered images with PNG references using `vello_test_support`.
It does not call the GPU renderer's device probe.

`run.sh` runs seven independent builds: no optional shader features, each of the five shader
features enabled alone, and all five enabled. Both depth-buffer configurations are rendered.

```sh
bash vello_tests/feature_tests/run.sh wgpu
bash vello_tests/feature_tests/run.sh webgl
```

Features are disabled by default. Select only this package when testing combinations; workspace
builds can enable additional features through other packages.

Each build filters the common probe elements to those supported by its flags. Solids, alpha
blending, nearest/bilinear images, filters, opacity/blend layers, transforms and depth buffering
are always included. The blurred rectangle, bicubic image and three gradient cells are included
only when their corresponding features are enabled. Universal elements come first, in the same
positions in every combination, followed by the enabled optional elements in common probe order.
The image source remains solid red.

Each combination has its own reference PNG in `vello_tests/snapshots`, shared by both GPU
backends. The native CPU f32 `probe_reference` test creates missing references and supports the
existing `REPLACE=1` workflow;
creating or replacing a reference intentionally fails that run for review. For example:

```sh
cargo test -p vello_gpu_feature_tests --no-default-features \
    --features gradient_radial --test features probe_reference
```

Generate references on native targets before compiling browser tests, which embed the PNGs.
Native failures write PNG and JSON diffs to this crate's `diffs` directory. Browser failures show
image diffs in the test page. Channel tolerances are 1 for CPU references and 3 for GPU output,
with no pixels allowed to exceed those thresholds.

CI runs the WGPU combinations in the existing Linux GPU job, and the WebGL combinations in
both existing browser jobs (default and SIMD128). They share the release profile, compiler
flags, target directory and cache with the existing tests; native release debug assertions
also stay enabled. WebGL reuses the installed tools and the existing browser timeout config.

Each shader variant still needs its own shader crate and GPU backend build.
