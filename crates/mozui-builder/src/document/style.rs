use mozui::{
    AbsoluteLength, AlignContent, AlignItems, DefiniteLength, Display, Fill, FlexDirection,
    FlexWrap, Hsla, Length, Overflow, Position, StyleRefinement, Visibility, px, rems,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct StyleDescriptor {
    // Display & layout
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display: Option<DisplayMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visibility: Option<VisibilityMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flex_direction: Option<FlexDir>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flex_wrap: Option<WrapMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flex_grow: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flex_shrink: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flex_basis: Option<LengthValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub align_items: Option<Align>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub align_content: Option<AlignContentMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub justify_content: Option<AlignContentMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub align_self: Option<Align>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gap: Option<SizeValue>,

    // Sizing
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<LengthValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<LengthValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_width: Option<LengthValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_height: Option<LengthValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_width: Option<LengthValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_height: Option<LengthValue>,

    // Spacing
    #[serde(skip_serializing_if = "Option::is_none")]
    pub margin: Option<EdgesValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub padding: Option<EdgesValue>,

    // Position
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<PositionMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inset: Option<EdgesValue>,

    // Borders
    #[serde(skip_serializing_if = "Option::is_none")]
    pub border_widths: Option<EdgesValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub border_color: Option<ColorValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub corner_radii: Option<CornersValue>,

    // Visual
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background: Option<ColorValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opacity: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overflow_x: Option<OverflowMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overflow_y: Option<OverflowMode>,

    // Text
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_size: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_weight: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_color: Option<ColorValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_height: Option<f32>,
}

// --- Supporting types ---

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum LengthValue {
    Auto,
    Px(f32),
    Rem(f32),
    Percent(f32),
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct EdgesValue {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top: Option<LengthValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub right: Option<LengthValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bottom: Option<LengthValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub left: Option<LengthValue>,
}

impl EdgesValue {
    pub fn all(value: LengthValue) -> Self {
        Self {
            top: Some(value.clone()),
            right: Some(value.clone()),
            bottom: Some(value.clone()),
            left: Some(value),
        }
    }

    pub fn symmetric(vertical: LengthValue, horizontal: LengthValue) -> Self {
        Self {
            top: Some(vertical.clone()),
            bottom: Some(vertical),
            left: Some(horizontal.clone()),
            right: Some(horizontal),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct CornersValue {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_left: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_right: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bottom_right: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bottom_left: Option<f32>,
}

impl CornersValue {
    pub fn all(value: f32) -> Self {
        Self {
            top_left: Some(value),
            top_right: Some(value),
            bottom_right: Some(value),
            bottom_left: Some(value),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct SizeValue {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<LengthValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<LengthValue>,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct ColorValue {
    pub h: f32,
    pub s: f32,
    pub l: f32,
    #[serde(default = "default_alpha")]
    pub a: f32,
}

fn default_alpha() -> f32 {
    1.0
}

impl ColorValue {
    pub fn new(h: f32, s: f32, l: f32, a: f32) -> Self {
        Self { h, s, l, a }
    }

    pub fn to_hsla(&self) -> Hsla {
        Hsla {
            h: self.h,
            s: self.s,
            l: self.l,
            a: self.a,
        }
    }
}

impl From<Hsla> for ColorValue {
    fn from(hsla: Hsla) -> Self {
        Self {
            h: hsla.h,
            s: hsla.s,
            l: hsla.l,
            a: hsla.a,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum DisplayMode {
    Flex,
    Grid,
    Block,
    None,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum VisibilityMode {
    Visible,
    Hidden,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum FlexDir {
    Row,
    Column,
    RowReverse,
    ColumnReverse,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum WrapMode {
    NoWrap,
    Wrap,
    WrapReverse,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Align {
    Start,
    End,
    FlexStart,
    FlexEnd,
    Center,
    Baseline,
    Stretch,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum AlignContentMode {
    Start,
    End,
    FlexStart,
    FlexEnd,
    Center,
    Stretch,
    SpaceBetween,
    SpaceEvenly,
    SpaceAround,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum PositionMode {
    Relative,
    Absolute,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum OverflowMode {
    Visible,
    Clip,
    Hidden,
    Scroll,
}

// --- Conversion to mozui types ---

fn to_length_scaled(v: &LengthValue, zoom: f32) -> Length {
    match v {
        LengthValue::Auto => Length::Auto,
        LengthValue::Px(n) => {
            Length::Definite(DefiniteLength::Absolute(AbsoluteLength::Pixels(px(*n * zoom))))
        }
        LengthValue::Rem(n) => {
            Length::Definite(DefiniteLength::Absolute(AbsoluteLength::Rems(rems(*n * zoom))))
        }
        LengthValue::Percent(n) => Length::Definite(DefiniteLength::Fraction(*n / 100.0)),
    }
}

fn to_definite_length_scaled(v: &LengthValue, zoom: f32) -> Option<DefiniteLength> {
    match v {
        LengthValue::Px(n) => Some(DefiniteLength::Absolute(AbsoluteLength::Pixels(px(*n * zoom)))),
        LengthValue::Rem(n) => Some(DefiniteLength::Absolute(AbsoluteLength::Rems(rems(*n * zoom)))),
        LengthValue::Percent(n) => Some(DefiniteLength::Fraction(*n / 100.0)),
        LengthValue::Auto => None,
    }
}

fn to_absolute_length_scaled(v: &LengthValue, zoom: f32) -> Option<AbsoluteLength> {
    match v {
        LengthValue::Px(n) => Some(AbsoluteLength::Pixels(px(*n * zoom))),
        LengthValue::Rem(n) => Some(AbsoluteLength::Rems(rems(*n * zoom))),
        _ => None,
    }
}

impl StyleDescriptor {
    pub fn to_style_refinement(&self) -> StyleRefinement {
        self.to_style_refinement_zoomed(1.0)
    }

    pub fn to_style_refinement_zoomed(&self, zoom: f32) -> StyleRefinement {
        let mut s = StyleRefinement::default();

        if let Some(display) = &self.display {
            s.display = Some(match display {
                DisplayMode::Flex => Display::Flex,
                DisplayMode::Grid => Display::Grid,
                DisplayMode::Block => Display::Block,
                DisplayMode::None => Display::None,
            });
        }

        if let Some(vis) = &self.visibility {
            s.visibility = Some(match vis {
                VisibilityMode::Visible => Visibility::Visible,
                VisibilityMode::Hidden => Visibility::Hidden,
            });
        }

        if let Some(dir) = &self.flex_direction {
            s.flex_direction = Some(match dir {
                FlexDir::Row => FlexDirection::Row,
                FlexDir::Column => FlexDirection::Column,
                FlexDir::RowReverse => FlexDirection::RowReverse,
                FlexDir::ColumnReverse => FlexDirection::ColumnReverse,
            });
        }

        if let Some(wrap) = &self.flex_wrap {
            s.flex_wrap = Some(match wrap {
                WrapMode::NoWrap => FlexWrap::NoWrap,
                WrapMode::Wrap => FlexWrap::Wrap,
                WrapMode::WrapReverse => FlexWrap::WrapReverse,
            });
        }

        if let Some(grow) = self.flex_grow {
            s.flex_grow = Some(grow);
        }

        if let Some(shrink) = self.flex_shrink {
            s.flex_shrink = Some(shrink);
        }

        if let Some(basis) = &self.flex_basis {
            s.flex_basis = Some(to_length_scaled(basis, zoom));
        }

        if let Some(align) = &self.align_items {
            s.align_items = Some(to_align_items(align));
        }

        if let Some(align) = &self.align_self {
            s.align_self = Some(to_align_items(align));
        }

        if let Some(align) = &self.align_content {
            s.align_content = Some(to_align_content(align));
        }

        if let Some(justify) = &self.justify_content {
            s.justify_content = Some(to_align_content(justify));
        }

        if let Some(gap) = &self.gap {
            if let Some(w) = gap.width.as_ref().and_then(|v| to_definite_length_scaled(v, zoom)) {
                s.gap.width = Some(w);
            }
            if let Some(h) = gap.height.as_ref().and_then(|v| to_definite_length_scaled(v, zoom)) {
                s.gap.height = Some(h);
            }
        }

        // Sizing
        if let Some(w) = &self.width {
            s.size.width = Some(to_length_scaled(w, zoom));
        }
        if let Some(h) = &self.height {
            s.size.height = Some(to_length_scaled(h, zoom));
        }
        if let Some(w) = &self.min_width {
            s.min_size.width = Some(to_length_scaled(w, zoom));
        }
        if let Some(h) = &self.min_height {
            s.min_size.height = Some(to_length_scaled(h, zoom));
        }
        if let Some(w) = &self.max_width {
            s.max_size.width = Some(to_length_scaled(w, zoom));
        }
        if let Some(h) = &self.max_height {
            s.max_size.height = Some(to_length_scaled(h, zoom));
        }

        // Spacing
        if let Some(m) = &self.margin {
            apply_edges_length_scaled(m, &mut s.margin, zoom);
        }
        if let Some(p) = &self.padding {
            apply_edges_definite_scaled(p, &mut s.padding, zoom);
        }

        // Position
        if let Some(pos) = &self.position {
            s.position = Some(match pos {
                PositionMode::Relative => Position::Relative,
                PositionMode::Absolute => Position::Absolute,
            });
        }
        if let Some(inset) = &self.inset {
            apply_edges_length_scaled(inset, &mut s.inset, zoom);
        }

        // Borders
        if let Some(bw) = &self.border_widths {
            apply_edges_absolute_scaled(bw, &mut s.border_widths, zoom);
        }
        if let Some(bc) = &self.border_color {
            s.border_color = Some(bc.to_hsla());
        }
        if let Some(cr) = &self.corner_radii {
            if let Some(v) = cr.top_left {
                s.corner_radii.top_left = Some(AbsoluteLength::Pixels(px(v * zoom)));
            }
            if let Some(v) = cr.top_right {
                s.corner_radii.top_right = Some(AbsoluteLength::Pixels(px(v * zoom)));
            }
            if let Some(v) = cr.bottom_right {
                s.corner_radii.bottom_right = Some(AbsoluteLength::Pixels(px(v * zoom)));
            }
            if let Some(v) = cr.bottom_left {
                s.corner_radii.bottom_left = Some(AbsoluteLength::Pixels(px(v * zoom)));
            }
        }

        // Visual
        if let Some(bg) = &self.background {
            s.background = Some(Fill::Color(bg.to_hsla().into()));
        }
        if let Some(opacity) = self.opacity {
            s.opacity = Some(opacity);
        }
        if let Some(ox) = &self.overflow_x {
            s.overflow.x = Some(to_overflow(ox));
        }
        if let Some(oy) = &self.overflow_y {
            s.overflow.y = Some(to_overflow(oy));
        }

        // Text
        if let Some(size) = self.font_size {
            s.text.font_size = Some(AbsoluteLength::Pixels(px(size * zoom)).into());
        }
        if let Some(weight) = self.font_weight {
            s.text.font_weight = Some(mozui::FontWeight(weight));
        }
        if let Some(color) = &self.text_color {
            s.text.color = Some(color.to_hsla());
        }
        if let Some(lh) = self.line_height {
            s.text.line_height = Some(DefiniteLength::Absolute(AbsoluteLength::Pixels(px(lh * zoom))));
        }

        s
    }
}

fn to_align_items(a: &Align) -> AlignItems {
    match a {
        Align::Start => AlignItems::Start,
        Align::End => AlignItems::End,
        Align::FlexStart => AlignItems::FlexStart,
        Align::FlexEnd => AlignItems::FlexEnd,
        Align::Center => AlignItems::Center,
        Align::Baseline => AlignItems::Baseline,
        Align::Stretch => AlignItems::Stretch,
    }
}

fn to_align_content(a: &AlignContentMode) -> AlignContent {
    match a {
        AlignContentMode::Start => AlignContent::Start,
        AlignContentMode::End => AlignContent::End,
        AlignContentMode::FlexStart => AlignContent::FlexStart,
        AlignContentMode::FlexEnd => AlignContent::FlexEnd,
        AlignContentMode::Center => AlignContent::Center,
        AlignContentMode::Stretch => AlignContent::Stretch,
        AlignContentMode::SpaceBetween => AlignContent::SpaceBetween,
        AlignContentMode::SpaceEvenly => AlignContent::SpaceEvenly,
        AlignContentMode::SpaceAround => AlignContent::SpaceAround,
    }
}

fn to_overflow(o: &OverflowMode) -> Overflow {
    match o {
        OverflowMode::Visible => Overflow::Visible,
        OverflowMode::Clip => Overflow::Clip,
        OverflowMode::Hidden => Overflow::Hidden,
        OverflowMode::Scroll => Overflow::Scroll,
    }
}

fn apply_edges_length_scaled(edges: &EdgesValue, target: &mut mozui::EdgesRefinement<Length>, zoom: f32) {
    if let Some(v) = &edges.top {
        target.top = Some(to_length_scaled(v, zoom));
    }
    if let Some(v) = &edges.right {
        target.right = Some(to_length_scaled(v, zoom));
    }
    if let Some(v) = &edges.bottom {
        target.bottom = Some(to_length_scaled(v, zoom));
    }
    if let Some(v) = &edges.left {
        target.left = Some(to_length_scaled(v, zoom));
    }
}

fn apply_edges_definite_scaled(edges: &EdgesValue, target: &mut mozui::EdgesRefinement<DefiniteLength>, zoom: f32) {
    if let Some(v) = &edges.top {
        if let Some(dl) = to_definite_length_scaled(v, zoom) {
            target.top = Some(dl);
        }
    }
    if let Some(v) = &edges.right {
        if let Some(dl) = to_definite_length_scaled(v, zoom) {
            target.right = Some(dl);
        }
    }
    if let Some(v) = &edges.bottom {
        if let Some(dl) = to_definite_length_scaled(v, zoom) {
            target.bottom = Some(dl);
        }
    }
    if let Some(v) = &edges.left {
        if let Some(dl) = to_definite_length_scaled(v, zoom) {
            target.left = Some(dl);
        }
    }
}

fn apply_edges_absolute_scaled(edges: &EdgesValue, target: &mut mozui::EdgesRefinement<AbsoluteLength>, zoom: f32) {
    if let Some(v) = &edges.top {
        if let Some(al) = to_absolute_length_scaled(v, zoom) {
            target.top = Some(al);
        }
    }
    if let Some(v) = &edges.right {
        if let Some(al) = to_absolute_length_scaled(v, zoom) {
            target.right = Some(al);
        }
    }
    if let Some(v) = &edges.bottom {
        if let Some(al) = to_absolute_length_scaled(v, zoom) {
            target.bottom = Some(al);
        }
    }
    if let Some(v) = &edges.left {
        if let Some(al) = to_absolute_length_scaled(v, zoom) {
            target.left = Some(al);
        }
    }
}
