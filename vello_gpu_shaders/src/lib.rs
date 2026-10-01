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

    #[test]
    fn image_bicubic_only_gates_bicubic_sampling() {
        for enabled in [false, true] {
            let mut compiler = Wesl::new("shaders");
            compiler.use_stripping(true);
            compiler.set_feature("blurred_rounded_rect", true);
            compiler.set_feature("image_bicubic", enabled);
            let source = compiler
                .compile(&"package::render".parse().unwrap())
                .expect("render shader links")
                .to_string();
            let module = wgsl::parse_str(&source).expect("linked WGSL parses");
            Validator::new(ValidationFlags::all(), Capabilities::all())
                .validate(&module)
                .expect("linked WGSL validates");

            let has_function = |name: &str| {
                module.functions.iter().any(|(_, function)| {
                    function.name.as_deref().is_some_and(|n| n.ends_with(name))
                })
            };
            assert_eq!(has_function("external_bicubic_sample"), enabled);
            assert_eq!(has_function("cubic_weights"), enabled);
            assert!(has_function("external_bilinear_sample"));
            assert!(has_function("sample_external_image"));
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
