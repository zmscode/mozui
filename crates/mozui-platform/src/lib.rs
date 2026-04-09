mod traits;

#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_arch = "wasm32")]
pub mod web;

pub use traits::*;

pub fn create_platform() -> Box<dyn Platform> {
    #[cfg(target_os = "macos")]
    {
        Box::new(macos::MacPlatform::new())
    }

    #[cfg(target_os = "windows")]
    {
        Box::new(windows::WinPlatform::new())
    }

    #[cfg(target_os = "linux")]
    {
        Box::new(linux::LinuxPlatform::new())
    }

    #[cfg(target_arch = "wasm32")]
    {
        Box::new(web::WebPlatform::new())
    }

    #[cfg(target_os = "ios")]
    {
        compile_error!("iOS platform backend not yet implemented. See TARGET.md Phase 2.");
    }

    #[cfg(target_os = "android")]
    {
        compile_error!("Android platform backend not yet implemented. See TARGET.md Phase 3.");
    }

    #[cfg(not(any(
        target_os = "macos",
        target_os = "windows",
        target_os = "linux",
        target_arch = "wasm32",
        target_os = "ios",
        target_os = "android"
    )))]
    {
        panic!("Unsupported platform. See TARGET.md for supported targets.");
    }
}
