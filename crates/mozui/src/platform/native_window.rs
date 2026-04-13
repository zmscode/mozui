//! Native window chrome and hosted-container contracts shared by mozui backends.
#![cfg_attr(not(target_os = "macos"), allow(dead_code))]

use crate::window::{FrameCallback, NativeCallbackDispatcher};
use crate::{App, Bounds, Pixels, Point, SharedString, Size, Window};
use std::ffi::c_void;
use std::rc::Rc;

fn schedule_no_args(
    handler: Rc<dyn Fn(&mut Window, &mut App)>,
    dispatcher: NativeCallbackDispatcher,
) -> Box<dyn Fn()> {
    Box::new(move || {
        let handler = handler.clone();
        let callback: FrameCallback = Box::new(move |window, cx| {
            handler(window, cx);
        });
        dispatcher.dispatch(callback);
    })
}

fn schedule_value<P: 'static>(
    handler: Rc<dyn Fn(P, &mut Window, &mut App)>,
    dispatcher: NativeCallbackDispatcher,
) -> Box<dyn Fn(P)> {
    Box::new(move |value| {
        let handler = handler.clone();
        let callback: FrameCallback = Box::new(move |window, cx| {
            handler(value, window, cx);
        });
        dispatcher.dispatch(callback);
    })
}

/// Opaque handle for a native popover currently shown by the backend.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NativePopoverHandle(pub(crate) usize);

/// Opaque handle for a native sheet currently shown by the backend.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NativeSheetHandle(pub(crate) usize);

/// Display mode for a native toolbar.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum NativeToolbarDisplayMode {
    /// Let the platform choose the best mode.
    #[default]
    Default,
    /// Show only toolbar item icons.
    IconOnly,
    /// Show only toolbar item labels.
    LabelOnly,
    /// Show both icons and labels where supported.
    IconAndLabel,
}

/// A native toolbar model installed on a window.
pub struct NativeToolbar {
    pub(crate) items: Vec<NativeToolbarItem>,
    pub(crate) display_mode: NativeToolbarDisplayMode,
    pub(crate) shows_title: bool,
}

impl NativeToolbar {
    /// Create an empty native toolbar.
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            display_mode: NativeToolbarDisplayMode::default(),
            shows_title: true,
        }
    }

    /// Set the toolbar display mode.
    pub fn display_mode(mut self, display_mode: NativeToolbarDisplayMode) -> Self {
        self.display_mode = display_mode;
        self
    }

    /// Control whether the window title is shown in the toolbar/titlebar area.
    pub fn shows_title(mut self, shows_title: bool) -> Self {
        self.shows_title = shows_title;
        self
    }

    /// Append a toolbar item.
    pub fn item(mut self, item: NativeToolbarItem) -> Self {
        self.items.push(item);
        self
    }

    pub(crate) fn into_platform(
        self,
        dispatcher: NativeCallbackDispatcher,
    ) -> PlatformNativeToolbar {
        PlatformNativeToolbar {
            items: self
                .items
                .into_iter()
                .map(|item| item.into_platform(dispatcher.clone()))
                .collect(),
            display_mode: self.display_mode,
            shows_title: self.shows_title,
        }
    }
}

impl Default for NativeToolbar {
    fn default() -> Self {
        Self::new()
    }
}

/// A single item in a native toolbar.
pub enum NativeToolbarItem {
    /// The standard AppKit sidebar toggle item.
    ToggleSidebar,
    /// The standard AppKit sidebar tracking separator item.
    SidebarTrackingSeparator,
    /// Flexible space between items.
    FlexibleSpace,
    /// Fixed space between items.
    Space,
    /// A clickable toolbar button.
    Button(NativeToolbarButton),
    /// A segmented toolbar group.
    Group(NativeToolbarGroup),
    /// A native toolbar search item.
    SearchField(NativeToolbarSearchField),
    /// A hosted native view container inside the toolbar.
    HostView(NativeToolbarHostView),
}

impl NativeToolbarItem {
    fn into_platform(self, dispatcher: NativeCallbackDispatcher) -> PlatformNativeToolbarItem {
        match self {
            Self::ToggleSidebar => PlatformNativeToolbarItem::ToggleSidebar,
            Self::SidebarTrackingSeparator => PlatformNativeToolbarItem::SidebarTrackingSeparator,
            Self::FlexibleSpace => PlatformNativeToolbarItem::FlexibleSpace,
            Self::Space => PlatformNativeToolbarItem::Space,
            Self::Button(button) => {
                PlatformNativeToolbarItem::Button(button.into_platform(dispatcher))
            }
            Self::Group(group) => PlatformNativeToolbarItem::Group(group.into_platform(dispatcher)),
            Self::SearchField(search) => {
                PlatformNativeToolbarItem::SearchField(search.into_platform(dispatcher))
            }
            Self::HostView(host_view) => PlatformNativeToolbarItem::HostView(host_view),
        }
    }
}

/// A clickable button in a native toolbar.
pub struct NativeToolbarButton {
    pub(crate) identifier: SharedString,
    pub(crate) symbol: SharedString,
    pub(crate) label: SharedString,
    pub(crate) navigational: bool,
    pub(crate) on_activate: Option<Rc<dyn Fn(&mut Window, &mut App)>>,
}

impl NativeToolbarButton {
    /// Create a native toolbar button.
    pub fn new(
        identifier: impl Into<SharedString>,
        symbol: impl Into<SharedString>,
        label: impl Into<SharedString>,
    ) -> Self {
        Self {
            identifier: identifier.into(),
            symbol: symbol.into(),
            label: label.into(),
            navigational: false,
            on_activate: None,
        }
    }

    /// Mark the item as navigational.
    pub fn navigational(mut self, navigational: bool) -> Self {
        self.navigational = navigational;
        self
    }

    /// Register a button activation callback.
    pub fn on_activate(mut self, listener: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_activate = Some(Rc::new(listener));
        self
    }

    fn into_platform(self, dispatcher: NativeCallbackDispatcher) -> PlatformNativeToolbarButton {
        PlatformNativeToolbarButton {
            identifier: self.identifier,
            symbol: self.symbol,
            label: self.label,
            navigational: self.navigational,
            on_activate: self
                .on_activate
                .map(|handler| schedule_no_args(handler, dispatcher)),
        }
    }
}

/// An event emitted when a native toolbar group changes selection.
#[derive(Clone, Debug)]
pub struct NativeToolbarGroupEvent {
    /// The selected segment index.
    pub selected_index: usize,
}

/// A segment inside a native toolbar group.
pub struct NativeToolbarGroupItem {
    pub(crate) symbol: SharedString,
    pub(crate) label: SharedString,
}

impl NativeToolbarGroupItem {
    /// Create a toolbar group segment.
    pub fn new(symbol: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            symbol: symbol.into(),
            label: label.into(),
        }
    }
}

/// A segmented control group in a native toolbar.
pub struct NativeToolbarGroup {
    pub(crate) identifier: SharedString,
    pub(crate) items: Vec<NativeToolbarGroupItem>,
    pub(crate) on_activate: Option<Rc<dyn Fn(NativeToolbarGroupEvent, &mut Window, &mut App)>>,
}

impl NativeToolbarGroup {
    /// Create a native toolbar group.
    pub fn new(identifier: impl Into<SharedString>) -> Self {
        Self {
            identifier: identifier.into(),
            items: Vec::new(),
            on_activate: None,
        }
    }

    /// Append a segment to the group.
    pub fn item(mut self, item: NativeToolbarGroupItem) -> Self {
        self.items.push(item);
        self
    }

    /// Register a group activation callback.
    pub fn on_activate(
        mut self,
        listener: impl Fn(NativeToolbarGroupEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_activate = Some(Rc::new(listener));
        self
    }

    fn into_platform(self, dispatcher: NativeCallbackDispatcher) -> PlatformNativeToolbarGroup {
        PlatformNativeToolbarGroup {
            identifier: self.identifier,
            items: self.items,
            on_activate: self.on_activate.map(|handler| {
                schedule_value(
                    Rc::new(move |selected_index, window, cx| {
                        handler(NativeToolbarGroupEvent { selected_index }, window, cx);
                    }),
                    dispatcher,
                )
            }),
        }
    }
}

/// An event emitted when a native search field changes.
#[derive(Clone, Debug)]
pub struct NativeSearchEvent {
    /// The current search text.
    pub text: SharedString,
}

/// A native toolbar search field item.
pub struct NativeToolbarSearchField {
    pub(crate) identifier: SharedString,
    pub(crate) placeholder: SharedString,
    pub(crate) on_change: Option<Rc<dyn Fn(NativeSearchEvent, &mut Window, &mut App)>>,
    pub(crate) on_submit: Option<Rc<dyn Fn(NativeSearchEvent, &mut Window, &mut App)>>,
}

impl NativeToolbarSearchField {
    /// Create a native toolbar search field item.
    pub fn new(identifier: impl Into<SharedString>) -> Self {
        Self {
            identifier: identifier.into(),
            placeholder: "Search".into(),
            on_change: None,
            on_submit: None,
        }
    }

    /// Set the placeholder text.
    pub fn placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    /// Register a live search callback.
    pub fn on_change(
        mut self,
        listener: impl Fn(NativeSearchEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Rc::new(listener));
        self
    }

    /// Register a submit callback.
    pub fn on_submit(
        mut self,
        listener: impl Fn(NativeSearchEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_submit = Some(Rc::new(listener));
        self
    }

    fn into_platform(
        self,
        dispatcher: NativeCallbackDispatcher,
    ) -> PlatformNativeToolbarSearchField {
        PlatformNativeToolbarSearchField {
            identifier: self.identifier,
            placeholder: self.placeholder,
            on_change: self.on_change.map(|handler| {
                schedule_value(
                    Rc::new(move |text: String, window, cx| {
                        handler(
                            NativeSearchEvent {
                                text: SharedString::from(text),
                            },
                            window,
                            cx,
                        );
                    }),
                    dispatcher.clone(),
                )
            }),
            on_submit: self.on_submit.map(|handler| {
                schedule_value(
                    Rc::new(move |text: String, window, cx| {
                        handler(
                            NativeSearchEvent {
                                text: SharedString::from(text),
                            },
                            window,
                            cx,
                        );
                    }),
                    dispatcher,
                )
            }),
        }
    }
}

/// A hosted native view container inside a toolbar item.
pub struct NativeToolbarHostView {
    pub(crate) identifier: SharedString,
    pub(crate) width: Pixels,
    pub(crate) height: Pixels,
    pub(crate) label: SharedString,
}

impl NativeToolbarHostView {
    /// Create a hosted native toolbar item view.
    pub fn new(identifier: impl Into<SharedString>, size: Size<Pixels>) -> Self {
        Self {
            identifier: identifier.into(),
            width: size.width,
            height: size.height,
            label: "".into(),
        }
    }

    /// Set the item label used by the native toolbar.
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = label.into();
        self
    }
}

/// A platform anchor used for native menus and popovers.
#[derive(Clone, Debug)]
pub enum NativeAnchor {
    /// Anchor to a content-space bounds rect.
    ContentBounds(Bounds<Pixels>),
    /// Anchor to a content-space point.
    ContentPoint(Point<Pixels>),
    /// Anchor to a toolbar item identifier.
    ToolbarItem(SharedString),
}

/// The presentation style for a native menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum NativeMenuKind {
    /// Standard popup/context menu.
    #[default]
    Popup,
    /// Suggestion/autocomplete menu.
    Suggestions,
}

/// A native popup or suggestion menu.
pub struct NativeMenu {
    pub(crate) kind: NativeMenuKind,
    pub(crate) anchor: NativeAnchor,
    pub(crate) items: Vec<NativeMenuItem>,
}

impl NativeMenu {
    /// Create a native menu.
    pub fn new(anchor: NativeAnchor) -> Self {
        Self {
            kind: NativeMenuKind::default(),
            anchor,
            items: Vec::new(),
        }
    }

    /// Set the menu kind.
    pub fn kind(mut self, kind: NativeMenuKind) -> Self {
        self.kind = kind;
        self
    }

    /// Append a menu item.
    pub fn item(mut self, item: NativeMenuItem) -> Self {
        self.items.push(item);
        self
    }

    pub(crate) fn into_platform(self, dispatcher: NativeCallbackDispatcher) -> PlatformNativeMenu {
        PlatformNativeMenu {
            kind: self.kind,
            anchor: self.anchor,
            items: self
                .items
                .into_iter()
                .map(|item| item.into_platform(dispatcher.clone()))
                .collect(),
        }
    }
}

/// A single item in a native menu.
pub struct NativeMenuItem {
    pub(crate) identifier: SharedString,
    pub(crate) title: SharedString,
    pub(crate) symbol: Option<SharedString>,
    pub(crate) enabled: bool,
    pub(crate) separator: bool,
    pub(crate) on_activate: Option<Rc<dyn Fn(&mut Window, &mut App)>>,
}

impl NativeMenuItem {
    /// Create a clickable menu item.
    pub fn new(identifier: impl Into<SharedString>, title: impl Into<SharedString>) -> Self {
        Self {
            identifier: identifier.into(),
            title: title.into(),
            symbol: None,
            enabled: true,
            separator: false,
            on_activate: None,
        }
    }

    /// Create a separator item.
    pub fn separator() -> Self {
        Self {
            identifier: "separator".into(),
            title: "".into(),
            symbol: None,
            enabled: false,
            separator: true,
            on_activate: None,
        }
    }

    /// Set an SF Symbol for the item when supported.
    pub fn symbol(mut self, symbol: impl Into<SharedString>) -> Self {
        self.symbol = Some(symbol.into());
        self
    }

    /// Enable or disable the item.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Register an activation callback.
    pub fn on_activate(mut self, listener: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_activate = Some(Rc::new(listener));
        self
    }

    fn into_platform(self, dispatcher: NativeCallbackDispatcher) -> PlatformNativeMenuItem {
        PlatformNativeMenuItem {
            identifier: self.identifier,
            title: self.title,
            symbol: self.symbol,
            enabled: self.enabled,
            separator: self.separator,
            on_activate: self
                .on_activate
                .map(|handler| schedule_no_args(handler, dispatcher)),
        }
    }
}

/// The preferred edge for a native popover.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum NativePopoverEdge {
    /// Arrow points upward from the anchored rect.
    Top,
    /// Arrow points to the left.
    Left,
    /// Arrow points downward from the anchored rect.
    #[default]
    Bottom,
    /// Arrow points to the right.
    Right,
}

/// Behavior for a native popover when users interact outside it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum NativePopoverBehavior {
    /// Closes when users click outside.
    #[default]
    Transient,
    /// Stays open until closed programmatically.
    ApplicationDefined,
    /// Closes when interacting with another window.
    Semitransient,
}

/// A native popover request.
pub struct NativePopover {
    pub(crate) anchor: NativeAnchor,
    pub(crate) size: Size<Pixels>,
    pub(crate) edge: NativePopoverEdge,
    pub(crate) behavior: NativePopoverBehavior,
    pub(crate) host_identifier: Option<SharedString>,
    pub(crate) on_close: Option<Rc<dyn Fn(&mut Window, &mut App)>>,
}

/// A native sheet request.
pub struct NativeSheet {
    pub(crate) size: Size<Pixels>,
    pub(crate) resizable: bool,
    pub(crate) host_identifier: Option<SharedString>,
    pub(crate) on_close: Option<Rc<dyn Fn(&mut Window, &mut App)>>,
}

impl NativeSheet {
    /// Create a native sheet with the requested content size.
    pub fn new(size: Size<Pixels>) -> Self {
        Self {
            size,
            resizable: true,
            host_identifier: None,
            on_close: None,
        }
    }

    /// Control whether the sheet window is resizable.
    pub fn resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }

    /// Create and expose a hosted native view for the sheet content.
    pub fn host_identifier(mut self, identifier: impl Into<SharedString>) -> Self {
        self.host_identifier = Some(identifier.into());
        self
    }

    /// Register a close callback.
    pub fn on_close(mut self, listener: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_close = Some(Rc::new(listener));
        self
    }

    pub(crate) fn into_platform(self, dispatcher: NativeCallbackDispatcher) -> PlatformNativeSheet {
        PlatformNativeSheet {
            size: self.size,
            resizable: self.resizable,
            host_identifier: self.host_identifier,
            on_close: self
                .on_close
                .map(|handler| schedule_no_args(handler, dispatcher)),
        }
    }
}

impl NativePopover {
    /// Create a native popover anchored to a toolbar item or content location.
    pub fn new(anchor: NativeAnchor, size: Size<Pixels>) -> Self {
        Self {
            anchor,
            size,
            edge: NativePopoverEdge::default(),
            behavior: NativePopoverBehavior::default(),
            host_identifier: None,
            on_close: None,
        }
    }

    /// Set the preferred arrow edge.
    pub fn edge(mut self, edge: NativePopoverEdge) -> Self {
        self.edge = edge;
        self
    }

    /// Set the popover behavior.
    pub fn behavior(mut self, behavior: NativePopoverBehavior) -> Self {
        self.behavior = behavior;
        self
    }

    /// Create and expose a hosted native view for the popover content.
    pub fn host_identifier(mut self, identifier: impl Into<SharedString>) -> Self {
        self.host_identifier = Some(identifier.into());
        self
    }

    /// Register a close callback.
    pub fn on_close(mut self, listener: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_close = Some(Rc::new(listener));
        self
    }

    pub(crate) fn into_platform(
        self,
        dispatcher: NativeCallbackDispatcher,
    ) -> PlatformNativePopover {
        PlatformNativePopover {
            anchor: self.anchor,
            size: self.size,
            edge: self.edge,
            behavior: self.behavior,
            host_identifier: self.host_identifier,
            on_close: self
                .on_close
                .map(|handler| schedule_no_args(handler, dispatcher)),
        }
    }
}

/// Configuration for a hosted native sidebar container.
pub struct NativeSidebarHost {
    pub(crate) identifier: SharedString,
    pub(crate) width: Pixels,
    pub(crate) min_width: Pixels,
    pub(crate) max_width: Pixels,
    pub(crate) visible: bool,
    pub(crate) overlays_content: bool,
    pub(crate) overlay_inset: Pixels,
    pub(crate) overlay_corner_radius: Pixels,
}

impl NativeSidebarHost {
    /// Create a hosted native sidebar container.
    pub fn new(identifier: impl Into<SharedString>, width: Pixels) -> Self {
        Self {
            identifier: identifier.into(),
            width,
            min_width: width,
            max_width: width,
            visible: true,
            overlays_content: false,
            overlay_inset: Pixels::ZERO,
            overlay_corner_radius: Pixels::ZERO,
        }
    }

    /// Set the minimum width.
    pub fn min_width(mut self, min_width: Pixels) -> Self {
        self.min_width = min_width;
        self
    }

    /// Set the maximum width.
    pub fn max_width(mut self, max_width: Pixels) -> Self {
        self.max_width = max_width;
        self
    }

    /// Set the initial visibility.
    pub fn visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }

    /// Allow the window content to continue underneath the sidebar host.
    ///
    /// On macOS this installs the sidebar as an overlay view instead of a split-view item.
    pub fn overlays_content(mut self, overlays_content: bool) -> Self {
        self.overlays_content = overlays_content;
        self
    }

    /// Apply a uniform inset around an overlay sidebar host.
    pub fn overlay_inset(mut self, overlay_inset: Pixels) -> Self {
        self.overlay_inset = overlay_inset;
        self
    }

    /// Apply corner rounding to an overlay sidebar host.
    pub fn overlay_corner_radius(mut self, overlay_corner_radius: Pixels) -> Self {
        self.overlay_corner_radius = overlay_corner_radius;
        self
    }
}

/// Configuration for a hosted native inspector container.
pub struct NativeInspectorHost {
    pub(crate) identifier: SharedString,
    pub(crate) width: Pixels,
    pub(crate) min_width: Pixels,
    pub(crate) max_width: Pixels,
    pub(crate) visible: bool,
}

impl NativeInspectorHost {
    /// Create a hosted native inspector container.
    pub fn new(identifier: impl Into<SharedString>, width: Pixels) -> Self {
        Self {
            identifier: identifier.into(),
            width,
            min_width: width,
            max_width: width,
            visible: false,
        }
    }

    /// Set the minimum width.
    pub fn min_width(mut self, min_width: Pixels) -> Self {
        self.min_width = min_width;
        self
    }

    /// Set the maximum width.
    pub fn max_width(mut self, max_width: Pixels) -> Self {
        self.max_width = max_width;
        self
    }

    /// Set the initial visibility.
    pub fn visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }
}

/// Which slot inside a native host should receive a mozui-managed hosted surface.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeHostedSurfaceTarget {
    /// The main body content of a native sidebar host.
    Sidebar,
    /// The header region of a native sidebar host.
    SidebarHeader,
    /// The main body content of a native inspector host.
    Inspector,
}

pub(crate) struct PlatformNativeToolbar {
    pub(crate) items: Vec<PlatformNativeToolbarItem>,
    pub(crate) display_mode: NativeToolbarDisplayMode,
    pub(crate) shows_title: bool,
}

pub(crate) enum PlatformNativeToolbarItem {
    ToggleSidebar,
    SidebarTrackingSeparator,
    FlexibleSpace,
    Space,
    Button(PlatformNativeToolbarButton),
    Group(PlatformNativeToolbarGroup),
    SearchField(PlatformNativeToolbarSearchField),
    HostView(NativeToolbarHostView),
}

pub(crate) struct PlatformNativeToolbarButton {
    pub(crate) identifier: SharedString,
    pub(crate) symbol: SharedString,
    pub(crate) label: SharedString,
    pub(crate) navigational: bool,
    pub(crate) on_activate: Option<Box<dyn Fn()>>,
}

pub(crate) struct PlatformNativeToolbarGroup {
    pub(crate) identifier: SharedString,
    pub(crate) items: Vec<NativeToolbarGroupItem>,
    pub(crate) on_activate: Option<Box<dyn Fn(usize)>>,
}

pub(crate) struct PlatformNativeToolbarSearchField {
    pub(crate) identifier: SharedString,
    pub(crate) placeholder: SharedString,
    pub(crate) on_change: Option<Box<dyn Fn(String)>>,
    pub(crate) on_submit: Option<Box<dyn Fn(String)>>,
}

pub(crate) struct PlatformNativeMenu {
    pub(crate) kind: NativeMenuKind,
    pub(crate) anchor: NativeAnchor,
    pub(crate) items: Vec<PlatformNativeMenuItem>,
}

pub(crate) struct PlatformNativeMenuItem {
    pub(crate) identifier: SharedString,
    pub(crate) title: SharedString,
    pub(crate) symbol: Option<SharedString>,
    pub(crate) enabled: bool,
    pub(crate) separator: bool,
    pub(crate) on_activate: Option<Box<dyn Fn()>>,
}

pub(crate) struct PlatformNativePopover {
    pub(crate) anchor: NativeAnchor,
    pub(crate) size: Size<Pixels>,
    pub(crate) edge: NativePopoverEdge,
    pub(crate) behavior: NativePopoverBehavior,
    pub(crate) host_identifier: Option<SharedString>,
    pub(crate) on_close: Option<Box<dyn Fn()>>,
}

pub(crate) struct PlatformNativeSheet {
    pub(crate) size: Size<Pixels>,
    pub(crate) resizable: bool,
    pub(crate) host_identifier: Option<SharedString>,
    pub(crate) on_close: Option<Box<dyn Fn()>>,
}

/// Platform-native window chrome interface implemented by backend window systems.
pub(crate) trait PlatformNativeWindow {
    /// Install or replace the native toolbar model for the window.
    fn set_native_toolbar(&self, _toolbar: PlatformNativeToolbar) {}

    /// Focus a native search item by identifier.
    fn focus_native_search_item(&self, _identifier: &str) -> bool {
        false
    }

    /// Show a native popup or suggestion menu.
    fn show_native_menu(&self, _menu: PlatformNativeMenu) -> bool {
        false
    }

    /// Show a native popover.
    fn show_native_popover(&self, _popover: PlatformNativePopover) -> Option<NativePopoverHandle> {
        None
    }

    /// Close a native popover shown earlier.
    fn close_native_popover(&self, _handle: NativePopoverHandle) -> bool {
        false
    }

    /// Return the raw native popover pointer for a shown popover handle.
    fn raw_native_popover_ptr(&self, _handle: NativePopoverHandle) -> Option<*mut c_void> {
        None
    }

    /// Show a native sheet.
    fn show_native_sheet(&self, _sheet: PlatformNativeSheet) -> Option<NativeSheetHandle> {
        None
    }

    /// Close a native sheet shown earlier.
    fn close_native_sheet(&self, _handle: NativeSheetHandle) -> bool {
        false
    }

    /// Install or update a hosted native sidebar container.
    fn install_native_sidebar_host(&self, _host: NativeSidebarHost) -> bool {
        false
    }

    /// Install or update a hosted native inspector container.
    fn install_native_inspector_host(&self, _host: NativeInspectorHost) -> bool {
        false
    }

    /// Update the visibility of a hosted native container.
    fn set_native_host_visibility(&self, _identifier: &str, _visible: bool) -> bool {
        false
    }

    /// Return the raw native host view pointer for a hosted container identifier.
    fn raw_native_host_view_ptr(&self, _identifier: &str) -> Option<*mut c_void> {
        None
    }

    /// Replace the hosted native content view for a container identifier.
    fn set_native_host_content(&self, _identifier: &str, _content_view: *mut c_void) -> bool {
        false
    }

    /// Attach a mozui-managed hosted surface view to a specific native host slot.
    ///
    /// This is the substrate needed for Glass-style native shells with custom
    /// mozui-rendered content mounted inside them.
    fn attach_native_hosted_surface(
        &self,
        _identifier: &str,
        _surface_view: *mut c_void,
        _target: NativeHostedSurfaceTarget,
    ) -> bool {
        false
    }

    /// Clear the hosted native content view for a container identifier.
    fn clear_native_host_content(&self, _identifier: &str) -> bool {
        false
    }
}
