use mozui_platform::PlatformWindow;

pub struct GpuContext {
    pub instance: wgpu::Instance,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
}

impl GpuContext {
    pub fn new_with_surface(window: &dyn PlatformWindow) -> (Self, wgpu::Surface<'static>) {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        // SAFETY: The window outlives the surface in our architecture.
        // We store the window alongside the surface to ensure this.
        let surface = unsafe {
            let raw_window = window.window_handle().expect("Window handle").as_raw();
            let raw_display = window.display_handle().expect("Display handle").as_raw();
            instance
                .create_surface_unsafe(wgpu::SurfaceTargetUnsafe::RawHandle {
                    raw_display_handle: raw_display,
                    raw_window_handle: raw_window,
                })
                .expect("Failed to create surface")
        };

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            force_fallback_adapter: false,
            compatible_surface: Some(&surface),
        }))
        .expect("No suitable GPU adapter found");

        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("mozui_device"),
            ..Default::default()
        }))
        .expect("Failed to create GPU device");

        (
            Self {
                instance,
                adapter,
                device,
                queue,
            },
            surface,
        )
    }
}
