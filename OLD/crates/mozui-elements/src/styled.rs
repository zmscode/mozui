/// Component size variants, matching gpui-component's Size enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ComponentSize {
    XSmall,
    Small,
    #[default]
    Medium,
    Large,
    Custom(u32), // arbitrary pixel-based size, stored as u32 for Eq/Hash
}

impl ComponentSize {
    pub fn smaller(&self) -> Self {
        match self {
            Self::Large => Self::Medium,
            Self::Medium => Self::Small,
            Self::Small => Self::XSmall,
            _ => Self::XSmall,
        }
    }

    pub fn larger(&self) -> Self {
        match self {
            Self::XSmall => Self::Small,
            Self::Small => Self::Medium,
            Self::Medium => Self::Large,
            _ => Self::Large,
        }
    }

    /// Standard input height for this size.
    pub fn input_height(&self) -> f32 {
        match self {
            Self::XSmall => 24.0,
            Self::Small => 28.0,
            Self::Medium => 32.0,
            Self::Large => 40.0,
            Self::Custom(px) => *px as f32,
        }
    }

    /// Standard font size for input text at this size.
    pub fn input_text_size(&self) -> f32 {
        match self {
            Self::XSmall => 11.0,
            Self::Small => 12.0,
            Self::Medium => 13.0,
            Self::Large => 15.0,
            Self::Custom(_) => 13.0,
        }
    }

    /// Standard horizontal padding for inputs at this size.
    pub fn input_px(&self) -> f32 {
        match self {
            Self::XSmall => 4.0,
            Self::Small => 6.0,
            Self::Medium => 8.0,
            Self::Large => 12.0,
            Self::Custom(_) => 8.0,
        }
    }

    /// Standard vertical padding for inputs at this size.
    pub fn input_py(&self) -> f32 {
        match self {
            Self::XSmall => 2.0,
            Self::Small => 3.0,
            Self::Medium => 4.0,
            Self::Large => 6.0,
            Self::Custom(_) => 4.0,
        }
    }

    /// Standard button text size for this size.
    pub fn button_text_size(&self) -> f32 {
        match self {
            Self::XSmall => 11.0,
            Self::Small => 12.0,
            Self::Medium => 13.0,
            Self::Large => 15.0,
            Self::Custom(_) => 13.0,
        }
    }

    /// Standard list item height for this size.
    pub fn list_item_height(&self) -> f32 {
        match self {
            Self::XSmall => 22.0,
            Self::Small => 26.0,
            Self::Medium => 30.0,
            Self::Large => 38.0,
            Self::Custom(px) => *px as f32,
        }
    }

    /// Standard list horizontal padding.
    pub fn list_px(&self) -> f32 {
        match self {
            Self::XSmall => 4.0,
            Self::Small => 6.0,
            Self::Medium => 8.0,
            Self::Large => 12.0,
            Self::Custom(_) => 8.0,
        }
    }

    /// Standard table row height for this size.
    pub fn table_row_height(&self) -> f32 {
        match self {
            Self::XSmall => 28.0,
            Self::Small => 32.0,
            Self::Medium => 38.0,
            Self::Large => 48.0,
            Self::Custom(px) => *px as f32,
        }
    }
}

impl From<f32> for ComponentSize {
    fn from(px: f32) -> Self {
        Self::Custom(px as u32)
    }
}

/// A component that can be sized.
pub trait Sizable: Sized {
    fn with_size(self, size: impl Into<ComponentSize>) -> Self;

    fn xsmall(self) -> Self {
        self.with_size(ComponentSize::XSmall)
    }

    fn small(self) -> Self {
        self.with_size(ComponentSize::Small)
    }

    fn large(self) -> Self {
        self.with_size(ComponentSize::Large)
    }
}

/// A component that can be disabled.
pub trait Disableable: Sized {
    fn disabled(self, disabled: bool) -> Self;
}

/// A component that can be selected.
pub trait Selectable: Sized {
    fn selected(self, selected: bool) -> Self;
}

/// A component that can be collapsed.
pub trait Collapsible: Sized {
    fn collapsed(self, collapsed: bool) -> Self;
}
