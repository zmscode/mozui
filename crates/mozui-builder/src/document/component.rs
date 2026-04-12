use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type")]
pub enum ComponentDescriptor {
    Container,

    Text {
        content: String,
    },

    Button {
        label: String,
        #[serde(default)]
        variant: ButtonVariant,
    },

    Input {
        #[serde(default)]
        placeholder: String,
    },

    Checkbox {
        #[serde(default)]
        label: String,
        #[serde(default)]
        checked: bool,
    },

    Badge {
        text: String,
    },

    Divider,

    Switch {
        #[serde(default)]
        label: String,
        #[serde(default)]
        checked: bool,
    },

    Progress {
        #[serde(default = "default_progress")]
        value: f32,
    },

    Avatar {
        #[serde(default)]
        name: String,
    },

    Label {
        text: String,
        #[serde(default)]
        description: String,
    },

    Radio {
        #[serde(default)]
        label: String,
        #[serde(default)]
        selected: bool,
    },
}

fn default_progress() -> f32 {
    50.0
}

impl ComponentDescriptor {
    pub fn display_name(&self) -> &str {
        match self {
            Self::Container => "Container",
            Self::Text { .. } => "Text",
            Self::Button { .. } => "Button",
            Self::Input { .. } => "Input",
            Self::Checkbox { .. } => "Checkbox",
            Self::Badge { .. } => "Badge",
            Self::Divider => "Divider",
            Self::Switch { .. } => "Switch",
            Self::Progress { .. } => "Progress",
            Self::Avatar { .. } => "Avatar",
            Self::Label { .. } => "Label",
            Self::Radio { .. } => "Radio",
        }
    }

    pub fn can_have_children(&self) -> bool {
        matches!(self, Self::Container)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub enum ButtonVariant {
    #[default]
    Primary,
    Secondary,
    Ghost,
    Danger,
}
