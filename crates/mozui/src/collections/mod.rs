#![allow(missing_docs)]
/// Standard collection types with FxHash defaults.
pub type HashMap<K, V> = FxHashMap<K, V>;
/// Standard collection types with FxHash defaults.
pub type HashSet<T> = FxHashSet<T>;
/// Indexed map with FxHash.
pub type IndexMap<K, V> = indexmap::IndexMap<K, V, rustc_hash::FxBuildHasher>;
/// Indexed set with FxHash.
pub type IndexSet<T> = indexmap::IndexSet<T, rustc_hash::FxBuildHasher>;

pub use indexmap::Equivalent;
pub use rustc_hash::FxHasher;
pub use rustc_hash::{FxHashMap, FxHashSet};
pub use std::collections::*;

pub mod vecmap;
