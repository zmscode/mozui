mod traits;

#[cfg(target_os = "macos")]
mod macos;

pub use traits::*;

pub fn create_platform() -> Box<dyn Platform> {
    #[cfg(target_os = "macos")]
    {
        Box::new(macos::MacPlatform::new())
    }
    #[cfg(not(target_os = "macos"))]
    {
        panic!("Unsupported platform. Currently only macOS is supported.");
    }
}
