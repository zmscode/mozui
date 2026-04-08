use std::any::Any;
use std::fmt::Debug;

/// Trait for named, dispatchable actions.
pub trait Action: Any + Debug {
    fn name(&self) -> &'static str;
    fn namespace(&self) -> &'static str;
    fn boxed_clone(&self) -> Box<dyn Action>;
    fn as_any(&self) -> &dyn Any;
}

/// Define action types in a namespace.
///
/// ```rust
/// actions!(editor, [Copy, Paste, Cut, Undo, Redo]);
/// ```
#[macro_export]
macro_rules! actions {
    ($namespace:ident, [$($action:ident),* $(,)?]) => {
        $(
            #[derive(Debug, Clone, Copy)]
            pub struct $action;

            impl $crate::Action for $action {
                fn name(&self) -> &'static str { stringify!($action) }
                fn namespace(&self) -> &'static str { stringify!($namespace) }
                fn boxed_clone(&self) -> Box<dyn $crate::Action> { Box::new(*self) }
                fn as_any(&self) -> &dyn std::any::Any { self }
            }
        )*
    };
}
