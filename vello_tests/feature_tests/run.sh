#!/usr/bin/env bash
set -euo pipefail

crate_dir=$(cd "$(dirname "$0")" && pwd)
backend=${1:-wgpu}
if [[ $backend != wgpu && $backend != webgl ]]; then
    echo "usage: $0 [wgpu|webgl] [additional cargo test arguments]" >&2
    exit 2
fi
if [[ $# -gt 0 ]]; then
    shift
fi

cargo_args=(--locked --no-default-features)
wasm_args=(--headless --chrome)
while [[ $# -gt 0 ]]; do
    case $1 in
        --release)
            if [[ $backend == webgl ]]; then
                wasm_args+=(--release)
            else
                cargo_args+=(--release)
            fi
            ;;
        --mode)
            if [[ $backend != webgl || $# -lt 2 ]]; then
                echo "--mode requires a value and the webgl backend" >&2
                exit 2
            fi
            wasm_args+=(--mode "$2")
            shift
            ;;
        *) cargo_args+=("$1") ;;
    esac
    shift
done

combinations=("" blurred_rounded_rect image_bicubic gradient_linear gradient_radial gradient_sweep all_shader_features)
for combination in "${combinations[@]}"; do
    features=$backend
    if [[ -n $combination ]]; then
        features+=,$combination
    fi
    echo "Testing $backend: ${combination:-no shader features}"
    if [[ $backend == wgpu ]]; then
        cargo test --manifest-path "$crate_dir/Cargo.toml" -p vello_gpu_feature_tests \
            --test features --features "$features" "${cargo_args[@]}"
    else
        wasm-pack test "${wasm_args[@]}" "$crate_dir" --test features \
            --features "$features" "${cargo_args[@]}"
    fi
done
