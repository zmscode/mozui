use mozui::{
    NativeToolbar, NativeToolbarButton, NativeToolbarDisplayMode, NativeToolbarGroup,
    NativeToolbarGroupItem, NativeToolbarItem, NativeToolbarSearchField, Window,
};
use std::rc::Rc;

/// Built-in toolbar item identifiers provided by AppKit.
pub enum ToolbarItemId {
    /// Toggle sidebar visibility (the standard sidebar button).
    ToggleSidebar,
    /// Tracking separator aligned with the sidebar split view divider.
    SidebarTrackingSeparator,
    /// Flexible space between items.
    FlexibleSpace,
    /// Fixed space between items.
    Space,
    /// Custom item with an SF Symbol icon.
    SymbolButton {
        id: String,
        symbol: String,
        label: String,
        /// If true, marks the item as navigational (left-side, own Liquid Glass pill).
        navigational: bool,
    },
    /// A group of SF Symbol buttons displayed as a segmented control.
    /// Items in a group share a single Liquid Glass pill.
    SymbolGroup {
        id: String,
        /// `(symbol_name, label)` pairs for each segment.
        items: Vec<(String, String)>,
    },
    /// A native toolbar search field item.
    SearchField {
        id: String,
        placeholder: String,
        on_change: Option<Rc<dyn Fn(String) + 'static>>,
        on_submit: Option<Rc<dyn Fn(String) + 'static>>,
    },
}

/// Installs an `NSToolbar` on the window associated with the given mozui `Window`.
///
/// This compatibility API now delegates to `mozui` core native-toolbar support
/// instead of owning a separate AppKit toolbar implementation.
pub fn install_toolbar(window: &Window, items: &[ToolbarItemId]) {
    let toolbar = items.iter().fold(
        NativeToolbar::new()
            .display_mode(NativeToolbarDisplayMode::IconOnly)
            .shows_title(true),
        |toolbar, item| toolbar.item(toolbar_item(item)),
    );
    window.set_native_toolbar(toolbar);
}

fn toolbar_item(item: &ToolbarItemId) -> NativeToolbarItem {
    match item {
        ToolbarItemId::ToggleSidebar => NativeToolbarItem::ToggleSidebar,
        ToolbarItemId::SidebarTrackingSeparator => NativeToolbarItem::SidebarTrackingSeparator,
        ToolbarItemId::FlexibleSpace => NativeToolbarItem::FlexibleSpace,
        ToolbarItemId::Space => NativeToolbarItem::Space,
        ToolbarItemId::SymbolButton {
            id,
            symbol,
            label,
            navigational,
        } => NativeToolbarItem::Button(
            NativeToolbarButton::new(id.clone(), symbol.clone(), label.clone())
                .navigational(*navigational),
        ),
        ToolbarItemId::SymbolGroup { id, items } => {
            let group = items
                .iter()
                .fold(NativeToolbarGroup::new(id.clone()), |group, item| {
                    group.item(NativeToolbarGroupItem::new(item.0.clone(), item.1.clone()))
                });
            NativeToolbarItem::Group(group)
        }
        ToolbarItemId::SearchField {
            id,
            placeholder,
            on_change,
            on_submit,
        } => {
            let search = NativeToolbarSearchField::new(id.clone()).placeholder(placeholder.clone());
            let search = if let Some(on_change) = on_change.clone() {
                search.on_change(move |event, _, _| on_change(event.text.to_string()))
            } else {
                search
            };
            let search = if let Some(on_submit) = on_submit.clone() {
                search.on_submit(move |event, _, _| on_submit(event.text.to_string()))
            } else {
                search
            };
            NativeToolbarItem::SearchField(search)
        }
    }
}
