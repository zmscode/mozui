use mozui::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, Render, Styled as _, Window, px,
};

use mozui_ui::{dock::PanelControl, text::markdown};

use crate::Story;

pub struct WelcomeStory {
    focus_handle: FocusHandle,
}

impl WelcomeStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
        }
    }
}

impl Story for WelcomeStory {
    fn title() -> &'static str {
        "Introduction"
    }

    fn description() -> &'static str {
        "UI components for building fantastic desktop application by using GPUI."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }

    fn zoomable() -> Option<PanelControl> {
        None
    }

    fn paddings() -> mozui::Pixels {
        px(0.)
    }
}

impl Focusable for WelcomeStory {
    fn focus_handle(&self, _: &mozui::App) -> mozui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for WelcomeStory {
    fn render(
        &mut self,
        _: &mut mozui::Window,
        _: &mut mozui::Context<Self>,
    ) -> impl mozui::IntoElement {
        markdown("# mozui\n\nA GPU-accelerated UI framework for Rust.")
            .px_4()
            .scrollable(true)
            .selectable(true)
    }
}
