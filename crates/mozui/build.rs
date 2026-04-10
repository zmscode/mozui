#![allow(clippy::disallowed_methods, reason = "build scripts are exempt")]

fn main() {
    println!("cargo::rustc-check-cfg=cfg(gles)");

    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();

    if target_os == "macos" {
        macos_build::run();
    }

    if target_os == "windows" {
        #[cfg(feature = "windows-manifest")]
        embed_resource();
    }
}

#[cfg(feature = "windows-manifest")]
fn embed_resource() {
    use std::io::Write;

    let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());

    let manifest_path = out_dir.join("mozui.manifest.xml");
    let mut f = std::fs::File::create(&manifest_path).unwrap();
    f.write_all(WINDOWS_MANIFEST.as_bytes()).unwrap();

    let rc_path = out_dir.join("mozui.rc");
    let mut f = std::fs::File::create(&rc_path).unwrap();
    writeln!(f, "#define RT_MANIFEST 24").unwrap();
    writeln!(f, "1 RT_MANIFEST \"{}\"", manifest_path.display()).unwrap();

    embed_resource::compile(&rc_path, embed_resource::NONE)
        .manifest_required()
        .unwrap();
}

#[cfg(feature = "windows-manifest")]
const WINDOWS_MANIFEST: &str = r#"<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
    <trustInfo xmlns="urn:schemas-microsoft-com:asm.v3">
        <security>
            <requestedPrivileges>
                <requestedExecutionLevel level="asInvoker" uiAccess="false" />
            </requestedPrivileges>
        </security>
    </trustInfo>
    <compatibility xmlns="urn:schemas-microsoft-com:compatibility.v1">
        <application>
            <!-- Windows 10 -->
            <supportedOS Id="{8e0f7a12-bfb3-4fe8-b9a5-48fd50a15a9a}" />
        </application>
    </compatibility>
    <application xmlns="urn:schemas-microsoft-com:asm.v3">
        <windowsSettings>
            <dpiAware xmlns="http://schemas.microsoft.com/SMI/2005/WindowsSettings">true/pm</dpiAware>
            <dpiAwareness xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">PerMonitorV2</dpiAwareness>
        </windowsSettings>
    </application>
    <dependency>
        <dependentAssembly>
            <assemblyIdentity
                type='win32'
                name='Microsoft.Windows.Common-Controls'
                version='6.0.0.0'
                processorArchitecture='*'
                publicKeyToken='6595b64144ccf1df'
            />
        </dependentAssembly>
    </dependency>
</assembly>"#;

mod macos_build {
    use std::{
        env,
        path::{Path, PathBuf},
    };

    pub fn run() {
        let header_path = generate_shader_bindings();
        generate_media_bindings();

        #[cfg(feature = "runtime_shaders")]
        emit_stitched_shaders(&header_path);
        #[cfg(not(feature = "runtime_shaders"))]
        compile_metal_shaders(&header_path);
    }

    fn generate_media_bindings() {
        use std::process::Command;

        let sdk_path = String::from_utf8(
            Command::new("xcrun")
                .args(["--sdk", "macosx", "--show-sdk-path"])
                .output()
                .unwrap()
                .stdout,
        )
        .unwrap();
        let sdk_path = sdk_path.trim_end();

        println!("cargo:rerun-if-changed=src/platform/macos/media_bindings.h");
        let bindings = bindgen::Builder::default()
            .header("src/platform/macos/media_bindings.h")
            .clang_arg(format!("-isysroot{}", sdk_path))
            .clang_arg("-xobjective-c")
            .allowlist_type("CMItemIndex")
            .allowlist_type("CMSampleTimingInfo")
            .allowlist_type("CMVideoCodecType")
            .allowlist_type("VTEncodeInfoFlags")
            .allowlist_function("CMTimeMake")
            .allowlist_var("kCVPixelFormatType_.*")
            .allowlist_var("kCVReturn.*")
            .allowlist_var("VTEncodeInfoFlags_.*")
            .allowlist_var("kCMVideoCodecType_.*")
            .allowlist_var("kCMTime.*")
            .allowlist_var("kCMSampleAttachmentKey_.*")
            .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
            .layout_tests(false)
            .generate()
            .expect("unable to generate media bindings");

        let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
        bindings
            .write_to_file(out_path.join("media_bindings.rs"))
            .expect("couldn't write media bindings");
    }

    fn generate_shader_bindings() -> PathBuf {
        use cbindgen::Config;

        let output_path = PathBuf::from(env::var("OUT_DIR").unwrap()).join("scene.h");

        let crate_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());

        let mut config = Config {
            include_guard: Some("SCENE_H".into()),
            language: cbindgen::Language::C,
            no_includes: true,
            ..Default::default()
        };
        config.export.include.extend([
            "Bounds".into(),
            "Corners".into(),
            "Edges".into(),
            "Size".into(),
            "Pixels".into(),
            "PointF".into(),
            "Hsla".into(),
            "ContentMask".into(),
            "Uniforms".into(),
            "AtlasTile".into(),
            "PathRasterizationInputIndex".into(),
            "PathVertex_ScaledPixels".into(),
            "PathRasterizationVertex".into(),
            "ShadowInputIndex".into(),
            "Shadow".into(),
            "QuadInputIndex".into(),
            "Underline".into(),
            "UnderlineInputIndex".into(),
            "Quad".into(),
            "BorderStyle".into(),
            "SpriteInputIndex".into(),
            "MonochromeSprite".into(),
            "PolychromeSprite".into(),
            "PathSprite".into(),
            "SurfaceInputIndex".into(),
            "SurfaceBounds".into(),
            "TransformationMatrix".into(),
        ]);
        config.no_includes = true;
        config.enumeration.prefix_with_name = true;

        let mut builder = cbindgen::Builder::new();

        // Source files that define types used in shaders
        let src_paths = [
            crate_dir.join("src/scene.rs"),
            crate_dir.join("src/geometry.rs"),
            crate_dir.join("src/color.rs"),
            crate_dir.join("src/window.rs"),
            crate_dir.join("src/platform.rs"),
            crate_dir.join("src/platform/macos/metal_renderer.rs"),
        ];

        for src_path in &src_paths {
            println!("cargo:rerun-if-changed={}", src_path.display());
            builder = builder.with_src(src_path);
        }

        builder
            .with_config(config)
            .generate()
            .expect("Unable to generate bindings")
            .write_to_file(&output_path);

        output_path
    }

    /// To enable runtime compilation, we need to "stitch" the shaders file with the generated header
    /// so that it is self-contained.
    #[cfg(feature = "runtime_shaders")]
    fn emit_stitched_shaders(header_path: &Path) {
        fn stitch_header(header: &Path, shader_path: &Path) -> std::io::Result<PathBuf> {
            let header_contents = std::fs::read_to_string(header)?;
            let shader_contents = std::fs::read_to_string(shader_path)?;
            let stitched_contents = format!("{header_contents}\n{shader_contents}");
            let out_path =
                PathBuf::from(env::var("OUT_DIR").unwrap()).join("stitched_shaders.metal");
            std::fs::write(&out_path, stitched_contents)?;
            Ok(out_path)
        }
        let shader_source_path = "./src/platform/macos/shaders.metal";
        let shader_path = PathBuf::from(shader_source_path);
        stitch_header(header_path, &shader_path).unwrap();
        println!("cargo:rerun-if-changed={}", &shader_source_path);
    }

    #[cfg(not(feature = "runtime_shaders"))]
    fn compile_metal_shaders(header_path: &Path) {
        use std::process::{self, Command};
        let shader_path = "./src/platform/macos/shaders.metal";
        let air_output_path = PathBuf::from(env::var("OUT_DIR").unwrap()).join("shaders.air");
        let metallib_output_path =
            PathBuf::from(env::var("OUT_DIR").unwrap()).join("shaders.metallib");
        println!("cargo:rerun-if-changed={}", shader_path);

        let output = Command::new("xcrun")
            .args([
                "-sdk",
                "macosx",
                "metal",
                "-gline-tables-only",
                "-mmacosx-version-min=10.15.7",
                "-MO",
                "-c",
                shader_path,
                "-include",
                (header_path.to_str().unwrap()),
                "-o",
            ])
            .arg(&air_output_path)
            .output()
            .unwrap();

        if !output.status.success() {
            println!(
                "cargo::error=metal shader compilation failed:\n{}",
                String::from_utf8_lossy(&output.stderr)
            );
            process::exit(1);
        }

        let output = Command::new("xcrun")
            .args(["-sdk", "macosx", "metallib"])
            .arg(air_output_path)
            .arg("-o")
            .arg(metallib_output_path)
            .output()
            .unwrap();

        if !output.status.success() {
            println!(
                "cargo::error=metallib compilation failed:\n{}",
                String::from_utf8_lossy(&output.stderr)
            );
            process::exit(1);
        }
    }
}
