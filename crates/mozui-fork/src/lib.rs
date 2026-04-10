// mozui-fork: GPUI + gpui-component unified re-export
//
// This crate is a thin wrapper that re-exports the GPUI framework from Zed
// alongside the gpui-component library, providing a single dependency for
// building GPU-accelerated desktop UIs with a full component set.

/// Re-export of the GPUI framework.
///
/// Provides the core UI primitives: elements, views, windows, layout,
/// text rendering, input handling, and the application lifecycle.
pub use gpui;

/// Re-export of the gpui_platform crate for direct platform access.
pub use gpui_platform;

/// Re-export of gpui_macros for derive macros (Render, IntoElement, etc).
pub use gpui_macros;

/// Re-export of the full gpui-component library.
///
/// Provides ready-to-use UI components: buttons, inputs, dialogs, menus,
/// tables, tabs, sidebars, charts, and more.
pub use gpui_component as components;

/// Re-export of the gpui-component bundled assets (fonts, icons, etc).
pub use gpui_component_assets as assets;

/// Re-export of gpui-component macros.
pub use gpui_component_macros as component_macros;

/// Convenience prelude that pulls in the most commonly used items from
/// both GPUI core and gpui-component.
///
/// GPUI core is re-exported first; gpui-component items that conflict
/// (e.g. color helper functions) are accessible via `components::*`.
pub mod prelude {
    pub use gpui::prelude::*;
    pub use gpui::*;

    // Re-export gpui-component's public items, excluding names that
    // conflict with gpui core (color helpers like `red`, `blue`, etc).
    pub use gpui_component::{
        accordion, alert, animation, avatar, badge, breadcrumb, button, chart, checkbox, clipboard,
        collapsible, color_picker, description_list, dialog, divider, dock, form, group_box,
        highlighter, history, hover_card, input, kbd, label, link, list, menu, notification,
        pagination, plot, popover, progress, radio, rating, resizable, scroll, select, setting,
        sheet, sidebar, skeleton, slider, spinner, stepper, switch, tab, table, tag, text, theme,
        tooltip, tree,
    };

    pub use gpui_component::{
        ActiveTheme, Disableable, ElementExt, FocusTrapElement, GlobalState, IndexPath, Root,
        Sizable, StyledExt, StyleSized, VirtualList, VirtualListScrollHandle, WindowBorder,
        WindowExt, h_flex, v_flex,
    };
}

/// Initialize mozui (calls gpui-component's init).
///
/// Must be called at application startup before using any components.
pub fn init(cx: &mut gpui::App) {
    gpui_component::init(cx);
}
