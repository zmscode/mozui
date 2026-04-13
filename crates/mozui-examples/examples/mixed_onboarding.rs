mod support;

use mozui::prelude::*;
use mozui::{
    ClickEvent, Context, Entity, Subscription, SymbolScale, SymbolWeight, Window, div, hsla,
    native_button, native_image_view, native_progress, native_switch, px, size,
};
use mozui_components::{
    Sizable, StyledExt as _,
    button::{Button, ButtonVariants},
    input::{Input, InputState},
    theme::ThemeMode,
};
use support::run_transparent_rooted_example;

/// Mixed onboarding example.
///
/// The window chrome uses `WindowBackgroundAppearance::Blurred` (via
/// `run_transparent_rooted_example`) for the system frosted-glass backdrop.
/// Inside, native leaf controls handle OS-level interactions:
///   - `native_image_view`   displays the step icon (SF Symbol)
///   - `native_switch`       controls OS-level permission toggles
///   - `native_button`       is the primary CTA
///   - `native_progress`     tracks setup progress
///
/// The form inputs and secondary back button stay on the `mozui-components` path.
///
/// Note on z-ordering: native NSViews always render above the mozui Metal surface.
/// This means native_glass_effect / native_visual_effect cannot be used as card
/// backgrounds when mozui content needs to sit visually on top — a styled div is
/// used for the card shell instead.
///
/// Run with:
///   cargo run -p mozui-examples --example mixed_onboarding
fn main() {
    run_transparent_rooted_example(
        "Mixed Onboarding",
        ThemeMode::Dark,
        size(px(640.0), px(760.0)),
        |window, cx| cx.new(|cx| MixedOnboardingExample::new(window, cx)),
    );
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Step {
    Welcome,
    Profile,
    Permissions,
    Done,
}

impl Step {
    fn index(self) -> usize {
        match self {
            Step::Welcome => 0,
            Step::Profile => 1,
            Step::Permissions => 2,
            Step::Done => 3,
        }
    }

    fn total() -> usize {
        4
    }

    fn progress(self) -> f64 {
        self.index() as f64 / (Self::total() - 1) as f64 * 100.0
    }

    fn symbol(self) -> &'static str {
        match self {
            Step::Welcome => "wand.and.stars",
            Step::Profile => "person.crop.circle.fill",
            Step::Permissions => "lock.shield.fill",
            Step::Done => "checkmark.seal.fill",
        }
    }

    fn title(self) -> &'static str {
        match self {
            Step::Welcome => "Welcome to Aurora",
            Step::Profile => "Create your profile",
            Step::Permissions => "Grant permissions",
            Step::Done => "You're all set",
        }
    }

    fn subtitle(self) -> &'static str {
        match self {
            Step::Welcome => {
                "Aurora keeps your work in sync across all your devices. Let's get you set up in a few quick steps."
            }
            Step::Profile => {
                "Tell Aurora a little about yourself so we can personalise your workspace."
            }
            Step::Permissions => {
                "Aurora needs a couple of system permissions to stay in sync while you work."
            }
            Step::Done => {
                "Your workspace is configured and ready. Everything is saved — you can always adjust these settings later."
            }
        }
    }

    fn next(self) -> Self {
        match self {
            Step::Welcome => Step::Profile,
            Step::Profile => Step::Permissions,
            Step::Permissions => Step::Done,
            Step::Done => Step::Done,
        }
    }

    fn back(self) -> Self {
        match self {
            Step::Welcome => Step::Welcome,
            Step::Profile => Step::Welcome,
            Step::Permissions => Step::Profile,
            Step::Done => Step::Permissions,
        }
    }
}

struct MixedOnboardingExample {
    step: Step,
    display_name: Entity<InputState>,
    email: Entity<InputState>,
    notifications_allowed: bool,
    background_sync: bool,
    _subscriptions: Vec<Subscription>,
}

impl MixedOnboardingExample {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let display_name = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Your name")
                .default_value("")
        });
        let email = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("you@example.com")
                .default_value("")
        });

        let subscriptions = vec![
            cx.observe(&display_name, |_, _, cx| cx.notify()),
            cx.observe(&email, |_, _, cx| cx.notify()),
        ];

        Self {
            step: Step::Welcome,
            display_name,
            email,
            notifications_allowed: true,
            background_sync: false,
            _subscriptions: subscriptions,
        }
    }

    fn go_next(&mut self, _: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>) {
        self.step = self.step.next();
        cx.notify();
    }

    fn go_back(&mut self, _: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>) {
        self.step = self.step.back();
        cx.notify();
    }

    fn set_notifications(
        &mut self,
        event: &mozui::SwitchChangeEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.notifications_allowed = event.checked;
        cx.notify();
    }

    fn set_background_sync(
        &mut self,
        event: &mozui::SwitchChangeEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.background_sync = event.checked;
        cx.notify();
    }
}

impl Render for MixedOnboardingExample {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let step = self.step;
        let is_first = step == Step::Welcome;
        let is_last = step == Step::Done;
        let progress = step.progress();

        div()
            .id("onboarding-root")
            .size_full()
            // Gradient backdrop so the blurred window has colour to show through.
            .bg(mozui::linear_gradient(
                160.,
                mozui::linear_color_stop(hsla(0.67, 0.38, 0.18, 1.0), 0.0),
                mozui::linear_color_stop(hsla(0.75, 0.28, 0.10, 1.0), 1.0),
            ))
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .p(px(32.0))
            // ---------------------------------------------------------------
            // Step card
            // ---------------------------------------------------------------
            .child(
                div()
                    .w(px(520.0))
                    .rounded(px(24.0))
                    .border_1()
                    .border_color(hsla(0.0, 0.0, 1.0, 0.12))
                    .bg(hsla(0.0, 0.0, 1.0, 0.08))
                    .flex()
                    .flex_col()
                    .gap(px(24.0))
                    .p(px(32.0))
                    // Step icon + title
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .items_center()
                            .gap(px(14.0))
                            // native_image_view fills its parent — wrap in a sized div.
                            .child(
                                div().w(px(56.0)).h(px(56.0)).child(
                                    native_image_view("onboarding-step-icon", step.symbol())
                                        .weight(SymbolWeight::Semibold)
                                        .scale(SymbolScale::Large)
                                        .point_size(42.0)
                                        .tint_color(0.72, 0.88, 1.0, 1.0),
                                ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .items_center()
                                    .gap(px(6.0))
                                    .child(
                                        div()
                                            .text_color(hsla(0.0, 0.0, 1.0, 0.96))
                                            .font_semibold()
                                            .text_sm()
                                            .child(step.title()),
                                    )
                                    .child(
                                        div()
                                            .text_color(hsla(0.0, 0.0, 1.0, 0.62))
                                            .text_xs()
                                            .text_center()
                                            .max_w(px(400.0))
                                            .child(step.subtitle()),
                                    ),
                            ),
                    )
                    // Step-specific content
                    .child(self.render_step_content(cx))
                    // Progress + navigation
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(14.0))
                            // Native progress bar tracks setup
                            .child(
                                native_progress("onboarding-progress")
                                    .range(0.0, 100.0)
                                    .value(progress)
                                    .h(px(4.0)),
                            )
                            .child(
                                div()
                                    .flex()
                                    .justify_between()
                                    .items_center()
                                    .child(
                                        // Secondary back button — mozui-components
                                        if !is_first {
                                            Button::new("onboarding-back")
                                                .label("Back")
                                                .secondary()
                                                .small()
                                                .on_click(cx.listener(Self::go_back))
                                                .into_any_element()
                                        } else {
                                            div().into_any_element()
                                        },
                                    )
                                    .child(
                                        // Primary CTA — native button
                                        native_button(
                                            "onboarding-next",
                                            if is_last { "Open Aurora" } else { "Continue" },
                                        )
                                        .button_style(mozui::NativeButtonStyle::Filled)
                                        .on_click(cx.listener(Self::go_next)),
                                    ),
                            ),
                    ),
            )
    }
}

impl MixedOnboardingExample {
    fn render_step_content(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let step = self.step;

        match step {
            Step::Welcome => div()
                .flex()
                .flex_col()
                .gap(px(10.0))
                .child(
                    div()
                        .rounded(px(14.0))
                        .border_1()
                        .border_color(hsla(0.0, 0.0, 1.0, 0.10))
                        .bg(hsla(0.0, 0.0, 1.0, 0.04))
                        .p(px(14.0))
                        .flex()
                        .flex_col()
                        .gap(px(8.0))
                        .child(feature_row("arrow.triangle.2.circlepath", "Live sync across all devices"))
                        .child(feature_row("waveform", "Real-time collaboration"))
                        .child(feature_row("lock.fill", "End-to-end encrypted storage")),
                )
                .into_any_element(),

            Step::Profile => div()
                .flex()
                .flex_col()
                .gap(px(12.0))
                .child(labeled_field(
                    "Display name",
                    Input::new(&self.display_name).small(),
                ))
                .child(labeled_field(
                    "Email address",
                    Input::new(&self.email).small(),
                ))
                .into_any_element(),

            Step::Permissions => div()
                .flex()
                .flex_col()
                .gap(px(12.0))
                // Native switch for an OS-level permission
                .child(permission_row(
                    "onboarding-perm-notifications",
                    "Notifications",
                    "Show alerts when your syncs complete or team members mention you.",
                    self.notifications_allowed,
                    cx.listener(Self::set_notifications),
                ))
                .child(permission_row(
                    "onboarding-perm-sync",
                    "Background sync",
                    "Keep files in sync even when Aurora is not in the foreground.",
                    self.background_sync,
                    cx.listener(Self::set_background_sync),
                ))
                .into_any_element(),

            Step::Done => div()
                .flex()
                .flex_col()
                .gap(px(10.0))
                .child(
                    div()
                        .rounded(px(14.0))
                        .border_1()
                        .border_color(hsla(0.45, 0.60, 0.60, 0.30))
                        .bg(hsla(0.45, 0.40, 0.40, 0.10))
                        .p(px(14.0))
                        .flex()
                        .flex_col()
                        .gap(px(6.0))
                        .child(
                            div()
                                .text_xs()
                                .font_semibold()
                                .text_color(hsla(0.0, 0.0, 1.0, 0.88))
                                .child("Setup complete"),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(hsla(0.0, 0.0, 1.0, 0.62))
                                .child("Aurora is syncing your workspace. You can close this window — everything is saved."),
                        ),
                )
                .into_any_element(),
        }
    }
}

// ---------------------------------------------------------------------------
// Layout helpers
// ---------------------------------------------------------------------------

fn labeled_field(label: &'static str, control: impl IntoElement) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap(px(5.0))
        .child(
            div()
                .text_xs()
                .font_semibold()
                .text_color(hsla(0.0, 0.0, 1.0, 0.72))
                .child(label),
        )
        .child(control)
}

fn feature_row(symbol: &'static str, label: &'static str) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap(px(10.0))
        // native_image_view fills its parent — wrap in a sized div.
        .child(
            div().w(px(20.0)).h(px(20.0)).child(
                native_image_view(format!("onboarding-feature-{symbol}"), symbol)
                    .weight(SymbolWeight::Semibold)
                    .scale(SymbolScale::Medium)
                    .point_size(14.0)
                    .tint_color(0.72, 0.88, 1.0, 1.0),
            ),
        )
        .child(
            div()
                .text_xs()
                .text_color(hsla(0.0, 0.0, 1.0, 0.82))
                .child(label),
        )
}

fn permission_row(
    id: &'static str,
    title: &'static str,
    description: &'static str,
    checked: bool,
    on_change: impl Fn(&mozui::SwitchChangeEvent, &mut Window, &mut mozui::App) + 'static,
) -> impl IntoElement {
    // Title and switch share the same line; description lives below.
    // This keeps the toggle visually anchored to the title regardless of
    // how much description text wraps.
    div()
        .rounded(px(14.0))
        .border_1()
        .border_color(hsla(0.0, 0.0, 1.0, 0.10))
        .bg(hsla(0.0, 0.0, 1.0, 0.04))
        .p(px(14.0))
        .flex()
        .flex_col()
        .gap(px(5.0))
        .child(
            // Title row: label on the left, native switch on the right.
            div()
                .flex()
                .justify_between()
                .items_center()
                .gap(px(16.0))
                .child(
                    div()
                        .text_xs()
                        .font_semibold()
                        .text_color(hsla(0.0, 0.0, 1.0, 0.88))
                        .child(title),
                )
                .child(
                    native_switch(id)
                        .checked(checked)
                        .on_change(on_change)
                        .flex_shrink_0(),
                ),
        )
        .child(
            div()
                .text_xs()
                .text_color(hsla(0.0, 0.0, 1.0, 0.56))
                .child(description),
        )
}
