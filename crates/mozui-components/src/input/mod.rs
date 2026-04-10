/// The character used to mask password input fields.
pub(super) const MASK_CHAR: char = '•';

mod blink_cursor;
mod change;
mod clear_button;
mod cursor;
mod display_map;
mod element;
mod indent;
mod input;
mod mask_pattern;
mod mode;
mod movement;
mod number_input;
mod otp_input;
mod rope_ext;
mod search;
mod selection;
mod state;

pub(crate) use clear_button::*;
pub use cursor::*;
pub use indent::TabSize;
pub use input::*;
pub use mask_pattern::MaskPattern;
pub use number_input::{NumberInput, NumberInputEvent, StepAction};
pub use otp_input::*;
pub use rope_ext::{InputEdit, Point, Position, RopeExt, RopeLines};
pub use ropey::Rope;
pub use state::*;
