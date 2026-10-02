// Copyright 2025 the Vello Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! WESL shader sources linked to WGSL and optionally compiled to GLSL for `vello_gpu`.

#[cfg(feature = "glsl")]
mod compile;
#[cfg(feature = "glsl")]
mod lint;
#[cfg(any(test, feature = "glsl"))]
mod minify;
#[cfg(feature = "glsl")]
mod types;

include!(concat!(env!("OUT_DIR"), "/compiled_shaders.rs"));

#[cfg(test)]
mod feature_tests {
    use naga::front::wgsl;
    use naga::valid::{Capabilities, ValidationFlags, Validator};
    use wesl::Wesl;

    fn render_module(features: &[(&str, bool)]) -> naga::Module {
        let mut compiler = Wesl::new("shaders");
        compiler.use_stripping(true);
        for feature in [
            "blurred_rounded_rect",
            "image_bicubic",
            "gradient_linear",
            "gradient_radial",
            "gradient_sweep",
        ] {
            compiler.set_feature(feature, true);
        }
        for &(feature, enabled) in features {
            compiler.set_feature(feature, enabled);
        }
        let source = compiler
            .compile(&"package::render".parse().unwrap())
            .expect("render shader links")
            .to_string();
        let module = wgsl::parse_str(&source).expect("linked WGSL parses");
        Validator::new(ValidationFlags::all(), Capabilities::all())
            .validate(&module)
            .expect("linked WGSL validates");
        #[cfg(feature = "glsl")]
        crate::compile::compile_wgsl_shader(
            &source,
            "render",
            "vs_main",
            "fs_main",
            &std::collections::BTreeMap::new(),
        );
        module
    }

    fn has_function(module: &naga::Module, name: &str) -> bool {
        module
            .functions
            .iter()
            .any(|(_, function)| function.name.as_deref().is_some_and(|n| n.ends_with(name)))
    }

    #[test]
    fn image_bicubic_only_gates_bicubic_sampling() {
        for enabled in [false, true] {
            let module = render_module(&[("image_bicubic", enabled)]);
            assert_eq!(has_function(&module, "external_bicubic_sample"), enabled);
            assert_eq!(has_function(&module, "cubic_weights"), enabled);
            assert!(has_function(&module, "external_bilinear_sample"));
            assert!(has_function(&module, "sample_external_image"));
        }
    }

    #[test]
    fn gradients_compile_independently_and_strip_unused_helpers() {
        for mask in 0..8 {
            let linear = mask & 1 != 0;
            let radial = mask & 2 != 0;
            let sweep = mask & 4 != 0;
            let any_gradient = linear || radial || sweep;
            let module = render_module(&[
                ("gradient_linear", linear),
                ("gradient_radial", radial),
                ("gradient_sweep", sweep),
            ]);
            for (name, expected) in [
                ("sample_linear_gradient", linear),
                ("sample_radial_gradient", radial),
                ("sample_sweep_gradient", sweep),
                ("calculate_radial_gradient", radial),
                ("xy_to_unit_angle", sweep),
                ("get_gradient_sample_xy", any_gradient),
                ("apply_gradient_transform", any_gradient),
                ("sample_gradient_lut", any_gradient),
            ] {
                assert_eq!(has_function(&module, name), expected, "{name}, mask={mask}");
            }
            let has_gradient_texture = module.global_variables.iter().any(|(_, global)| {
                global.binding
                    == Some(naga::ResourceBinding {
                        group: 3,
                        binding: 0,
                    })
            });
            assert_eq!(has_gradient_texture, any_gradient, "mask={mask}");
            assert!(has_function(&module, "sample_external_image"));
            assert!(has_function(&module, "sample_blurred_rounded_rect"));
        }
    }
}

#[cfg(all(test, feature = "glsl"))]
mod tests {
    use naga::front::wgsl;

    use crate::lint::lint;

    #[test]
    fn every_shipped_shader_passes_the_lint() {
        assert!(
            !crate::wgsl::ALL.is_empty(),
            "expected at least one linked WESL shader"
        );
        for &(name, source) in crate::wgsl::ALL {
            let module = wgsl::parse_str(source).expect("linked WGSL parses");
            lint(name, &module);
        }
    }
}
