/// A hash map using the FxHash algorithm for faster hashing.
pub type HashMap<K, V> = FxHashMap<K, V>;
/// A hash set using the FxHash algorithm for faster hashing.
pub type HashSet<T> = FxHashSet<T>;
/// An insertion-ordered map using FxHash.
pub type IndexMap<K, V> = indexmap::IndexMap<K, V, rustc_hash::FxBuildHasher>;
/// An insertion-ordered set using FxHash.
pub type IndexSet<T> = indexmap::IndexSet<T, rustc_hash::FxBuildHasher>;

pub use indexmap::Equivalent;
pub use rustc_hash::FxHasher;
pub use rustc_hash::{FxHashMap, FxHashSet};
pub use std::collections::*;

/// A vector-backed map for small collections.
pub mod vecmap;
