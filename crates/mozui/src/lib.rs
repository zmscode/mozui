// ── App framework ──────────────────────────────────────────────
pub use mozui_app::history::{History, HistoryItem};
pub use mozui_app::{Action, App, Context, KeyCombo, KeybindingRegistry, TimerId};
pub use mozui_app::{
    Cancel, Confirm, SelectDown, SelectFirst, SelectLast, SelectLeft, SelectNextColumn,
    SelectPageDown, SelectPageUp, SelectPrevColumn, SelectRight, SelectUp,
};

// Re-export the actions! macro
pub use mozui_app::actions;

// ── Elements ───────────────────────────────────────────────────
pub use mozui_elements::Root;
pub use mozui_elements::{Accordion, AccordionItem, accordion, accordion_item};
pub use mozui_elements::{Badge, badge};
pub use mozui_elements::{Breadcrumb, BreadcrumbItem, breadcrumb, breadcrumb_item};
pub use mozui_elements::{Button, ButtonGroup, ButtonVariant, button, button_group, icon_button};
pub use mozui_elements::{Checkbox, checkbox};
pub use mozui_elements::{Collapsible, ComponentSize, Disableable, Selectable, Sizable};
pub use mozui_elements::{CollapsibleContainer, collapsible};
pub use mozui_elements::{DescriptionItem, DescriptionList, description_item, description_list};
pub use mozui_elements::{
    Div, Element, ScrollOffset, Text, TextInput, TextInputState, div, text, text_input,
};
pub use mozui_elements::{Divider, DividerDirection, divider};
pub use mozui_elements::{FitMode, Popover};
pub use mozui_elements::{GroupBox, group_box};
pub use mozui_elements::{Icon, icon};
pub use mozui_elements::{Kbd, kbd};
pub use mozui_elements::{Label, LabelHighlight, LabelHighlightMode, label};
pub use mozui_elements::{Link, link};
pub use mozui_elements::{List, ListItem, list, list_item};
pub use mozui_elements::{Pagination, pagination};
pub use mozui_elements::{Progress, progress};
pub use mozui_elements::{Radio, radio};
pub use mozui_elements::{Rating, rating};
pub use mozui_elements::{Slider, slider};
pub use mozui_elements::{Stepper, StepperItem, stepper};
pub use mozui_elements::{Switch, switch};
pub use mozui_elements::{Tab, TabBar, tab, tab_bar};
pub use mozui_elements::{Tag, TagVariant, tag};
pub use mozui_elements::{VirtualList, VirtualListDirection};

// ── Icons ─────────────────────────────────────────────────────
pub use mozui_icons::{IconName, IconWeight};

// ── Events ─────────────────────────────────────────────────────
pub use mozui_events::{Key, Modifiers};

// ── Platform ───────────────────────────────────────────────────
pub use mozui_platform::{TitlebarStyle, WindowOptions, open_url};

// ── Reactivity ─────────────────────────────────────────────────
pub use mozui_reactive::{SetSignal, Signal};

// ── Rendering ──────────────────────────────────────────────────
pub use mozui_renderer::{DrawList, Renderer};

// ── Style ──────────────────────────────────────────────────────
pub use mozui_style::animation;
pub use mozui_style::animation::{Animated, Lerp, SpringHandle, Transition};
pub use mozui_style::palette;
pub use mozui_style::{
    Anchor, Axis, Color, ColorName, Corners, Edges, Fill, Placement, Point, Rect, Shadow, Side,
    Size, Style, Theme, ThemeMode,
};

pub use tracing_subscriber;
