use crate::platform::wgpu::{self, GpuContext, WgpuRenderer, WgpuSurfaceConfig};
use crate::{DevicePixels, GpuSpecs, PlatformAtlas, Scene, Size, WindowBackgroundAppearance};
use raw_window_handle::{DisplayHandle, HasDisplayHandle, HasWindowHandle, UiKitWindowHandle};
use std::{cell::RefCell, ffi::c_void, fmt, ptr::NonNull, rc::Rc, sync::Arc};

#[derive(Clone, Copy)]
pub(crate) struct IosRawWindow {
    ui_view: NonNull<c_void>,
    ui_view_controller: Option<NonNull<c_void>>,
}

impl IosRawWindow {
    #[allow(dead_code)]
    pub(crate) fn new(
        ui_view: NonNull<c_void>,
        ui_view_controller: Option<NonNull<c_void>>,
    ) -> Self {
        Self {
            ui_view,
            ui_view_controller,
        }
    }

    pub(crate) fn ui_view(self) -> NonNull<c_void> {
        self.ui_view
    }

    pub(crate) fn ui_view_controller(self) -> Option<NonNull<c_void>> {
        self.ui_view_controller
    }
}

impl fmt::Debug for IosRawWindow {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("IosRawWindow")
            .field("ui_view", &self.ui_view)
            .field("ui_view_controller", &self.ui_view_controller)
            .finish()
    }
}

// Safety: The raw UIKit pointers remain owned by the host iOS view hierarchy.
// They are passed to wgpu only for surface creation and are never dereferenced
// off-thread in Rust.
unsafe impl Send for IosRawWindow {}
unsafe impl Sync for IosRawWindow {}

impl HasWindowHandle for IosRawWindow {
    fn window_handle(
        &self,
    ) -> Result<raw_window_handle::WindowHandle<'_>, raw_window_handle::HandleError> {
        let mut handle = UiKitWindowHandle::new(self.ui_view);
        handle.ui_view_controller = self.ui_view_controller;
        Ok(unsafe { raw_window_handle::WindowHandle::borrow_raw(handle.into()) })
    }
}

impl HasDisplayHandle for IosRawWindow {
    fn display_handle(
        &self,
    ) -> Result<raw_window_handle::DisplayHandle<'_>, raw_window_handle::HandleError> {
        Ok(DisplayHandle::uikit())
    }
}

pub(crate) struct IosMetalRenderer {
    gpu_context: GpuContext,
    raw_window: IosRawWindow,
    renderer: WgpuRenderer,
}

impl IosMetalRenderer {
    pub(crate) fn new(
        raw_window: IosRawWindow,
        size: Size<DevicePixels>,
        background_appearance: WindowBackgroundAppearance,
    ) -> anyhow::Result<Self> {
        let gpu_context = Rc::new(RefCell::new(None));
        let renderer = WgpuRenderer::new(
            gpu_context.clone(),
            &raw_window,
            surface_config(size, background_appearance),
            None,
        )?;

        Ok(Self {
            gpu_context,
            raw_window,
            renderer,
        })
    }

    pub(crate) fn replace_surface(
        &mut self,
        raw_window: IosRawWindow,
        size: Size<DevicePixels>,
        background_appearance: WindowBackgroundAppearance,
    ) -> anyhow::Result<()> {
        let instance = self
            .gpu_context
            .borrow()
            .as_ref()
            .map(|context| context.instance.clone())
            .ok_or_else(|| {
                anyhow::anyhow!("iOS renderer surface cannot be replaced before initialization")
            })?;

        self.raw_window = raw_window;
        self.renderer.replace_surface(
            &self.raw_window,
            surface_config(size, background_appearance),
            &instance,
        )
    }

    pub(crate) fn suspend_surface(&mut self) {
        self.renderer.unconfigure_surface();
    }

    pub(crate) fn update_drawable_size(&mut self, size: Size<DevicePixels>) {
        self.renderer.update_drawable_size(size);
    }

    pub(crate) fn update_transparency(
        &mut self,
        background_appearance: WindowBackgroundAppearance,
    ) {
        self.renderer.update_transparency(!matches!(
            background_appearance,
            WindowBackgroundAppearance::Opaque
        ));
    }

    pub(crate) fn draw(&mut self, scene: &Scene) {
        if self.renderer.device_lost() {
            if let Err(error) = self.renderer.recover(&self.raw_window) {
                log::error!("failed to recover iOS Metal renderer after device loss: {error:#}");
                return;
            }
        }

        self.renderer.draw(scene);
    }

    pub(crate) fn sprite_atlas(&self) -> Arc<dyn PlatformAtlas> {
        self.renderer.sprite_atlas().clone()
    }

    pub(crate) fn gpu_specs(&self) -> GpuSpecs {
        self.renderer.gpu_specs()
    }

    pub(crate) fn destroy(&mut self) {
        self.renderer.destroy();
    }
}

fn surface_config(
    size: Size<DevicePixels>,
    background_appearance: WindowBackgroundAppearance,
) -> WgpuSurfaceConfig {
    WgpuSurfaceConfig {
        size,
        transparent: !matches!(background_appearance, WindowBackgroundAppearance::Opaque),
        // Prefer FIFO for the initial iOS bring-up because it is the most
        // predictable mode across simulator and device lifecycle transitions.
        preferred_present_mode: Some(wgpu::wgpu::PresentMode::Fifo),
    }
}
