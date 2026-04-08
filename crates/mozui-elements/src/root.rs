use crate::Element;
use mozui_style::Placement;

/// Type alias for overlay builder functions.
pub type OverlayBuilder = Box<dyn Fn(&mut dyn std::any::Any) -> Box<dyn Element>>;

/// An active dialog layer.
pub struct DialogLayer {
    pub id: usize,
    pub builder: OverlayBuilder,
}

/// An active sheet layer.
pub struct SheetLayer {
    pub id: usize,
    pub placement: Placement,
    pub builder: OverlayBuilder,
}

/// A notification entry.
pub struct Notification {
    pub id: usize,
    pub builder: OverlayBuilder,
}

/// Root element that manages the main view plus overlay layers
/// (dialogs, sheets, notifications) stacked on top.
///
/// This is the outermost element of a window, matching gpui-component's Root.
pub struct Root {
    next_id: usize,
    dialogs: Vec<DialogLayer>,
    sheets: Vec<SheetLayer>,
    notifications: Vec<Notification>,
}

impl Root {
    pub fn new() -> Self {
        Self {
            next_id: 0,
            dialogs: Vec::new(),
            sheets: Vec::new(),
            notifications: Vec::new(),
        }
    }

    fn next_id(&mut self) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    // ── Dialog management ──────────────────────────────────────────

    /// Open a modal dialog. Returns its ID.
    pub fn open_dialog(
        &mut self,
        builder: impl Fn(&mut dyn std::any::Any) -> Box<dyn Element> + 'static,
    ) -> usize {
        let id = self.next_id();
        self.dialogs.push(DialogLayer {
            id,
            builder: Box::new(builder),
        });
        id
    }

    /// Close the topmost dialog.
    pub fn close_dialog(&mut self) {
        self.dialogs.pop();
    }

    /// Close a specific dialog by ID.
    pub fn close_dialog_by_id(&mut self, id: usize) {
        self.dialogs.retain(|d| d.id != id);
    }

    /// Close all dialogs.
    pub fn close_all_dialogs(&mut self) {
        self.dialogs.clear();
    }

    /// Check if any dialog is active.
    pub fn has_active_dialog(&self) -> bool {
        !self.dialogs.is_empty()
    }

    // ── Sheet management ───────────────────────────────────────────

    /// Open a sheet at the given placement. Returns its ID.
    pub fn open_sheet(
        &mut self,
        placement: Placement,
        builder: impl Fn(&mut dyn std::any::Any) -> Box<dyn Element> + 'static,
    ) -> usize {
        let id = self.next_id();
        self.sheets.push(SheetLayer {
            id,
            placement,
            builder: Box::new(builder),
        });
        id
    }

    /// Close the active sheet.
    pub fn close_sheet(&mut self) {
        self.sheets.pop();
    }

    /// Close a specific sheet by ID.
    pub fn close_sheet_by_id(&mut self, id: usize) {
        self.sheets.retain(|s| s.id != id);
    }

    /// Check if any sheet is active.
    pub fn has_active_sheet(&self) -> bool {
        !self.sheets.is_empty()
    }

    /// Get the placement of the topmost sheet, if any.
    pub fn active_sheet_placement(&self) -> Option<Placement> {
        self.sheets.last().map(|s| s.placement)
    }

    // ── Notification management ────────────────────────────────────

    /// Push a notification. Returns its ID.
    pub fn push_notification(
        &mut self,
        builder: impl Fn(&mut dyn std::any::Any) -> Box<dyn Element> + 'static,
    ) -> usize {
        let id = self.next_id();
        self.notifications.push(Notification {
            id,
            builder: Box::new(builder),
        });
        id
    }

    /// Remove a notification by ID.
    pub fn remove_notification(&mut self, id: usize) {
        self.notifications.retain(|n| n.id != id);
    }

    /// Clear all notifications.
    pub fn clear_notifications(&mut self) {
        self.notifications.clear();
    }

    /// Number of active notifications.
    pub fn notification_count(&self) -> usize {
        self.notifications.len()
    }

    // ── Rendering helpers ──────────────────────────────────────────

    /// Build overlay elements from the current state.
    /// Call this during the render phase, passing the context.
    /// Returns a list of overlay elements to paint on top of the main view.
    pub fn build_overlays(&self, cx: &mut dyn std::any::Any) -> Vec<Box<dyn Element>> {
        let mut overlays = Vec::new();

        // Sheets
        for sheet in &self.sheets {
            overlays.push((sheet.builder)(cx));
        }

        // Dialogs (with overlay backdrop)
        for dialog in &self.dialogs {
            overlays.push((dialog.builder)(cx));
        }

        // Notifications
        for notification in &self.notifications {
            overlays.push((notification.builder)(cx));
        }

        overlays
    }
}

impl Default for Root {
    fn default() -> Self {
        Self::new()
    }
}
