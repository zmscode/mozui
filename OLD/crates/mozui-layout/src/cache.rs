use crate::LayoutId;
use std::collections::HashMap;
use std::fmt::Write;

/// Cache key for a layout subtree. Composed of a style hash
/// (from the taffy Style bytes) and a children hash (from child LayoutIds).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LayoutCacheKey {
    pub style_hash: u64,
    pub children_hash: u64,
}

struct CacheEntry {
    layout_id: LayoutId,
    generation: u64,
}

/// Frame-persistent layout cache. Skips taffy node creation for
/// subtrees whose style and children haven't changed since last frame.
///
/// Works with `LayoutEngine::begin_frame()` which preserves taffy nodes
/// across frames so cached LayoutIds remain valid.
pub struct LayoutCache {
    entries: HashMap<LayoutCacheKey, CacheEntry>,
    generation: u64,
}

impl LayoutCache {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            generation: 0,
        }
    }

    /// Start a new frame. Increments generation counter.
    /// Periodically evicts stale entries (every 120 frames).
    pub fn begin_frame(&mut self) {
        self.generation += 1;
        if self.generation % 120 == 0 {
            let cutoff = self.generation.saturating_sub(120);
            self.entries.retain(|_, e| e.generation >= cutoff);
        }
    }

    /// Look up a cached LayoutId for the given key.
    /// Marks the entry as used this frame if found.
    pub fn get(&mut self, key: LayoutCacheKey) -> Option<LayoutId> {
        if let Some(entry) = self.entries.get_mut(&key) {
            entry.generation = self.generation;
            Some(entry.layout_id)
        } else {
            None
        }
    }

    /// Store a LayoutId in the cache for future frames.
    pub fn insert(&mut self, key: LayoutCacheKey, layout_id: LayoutId) {
        self.entries.insert(
            key,
            CacheEntry {
                layout_id,
                generation: self.generation,
            },
        );
    }

    /// Clear all cached entries. Called during periodic full GC.
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

/// Hash a taffy Style via its Debug representation (FNV-1a).
/// taffy::Style doesn't implement Hash (its fields contain f32),
/// so we hash the deterministic Debug output instead.
pub fn hash_style(style: &taffy::Style) -> u64 {
    let mut buf = String::new();
    let _ = write!(buf, "{:?}", style);
    fnv1a(buf.as_bytes())
}

/// Hash a slice of child LayoutIds into a children_hash.
pub fn hash_children(children: &[LayoutId]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for child in children {
        let id = child.0 as u64;
        for &b in &id.to_ne_bytes() {
            hash ^= b as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
    }
    hash
}

fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &b in bytes {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}
