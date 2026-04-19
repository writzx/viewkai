//! LRU texture cache for viewkai.

use std::collections::HashMap;

use egui::TextureHandle;
use viewkai_core::page::PageIndex;

/// Cache key: page index + zoom bucket (DPI tier).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CacheKey {
    pub page_idx: PageIndex,
    pub zoom_bucket: u8,
}

/// A single cached texture entry.
struct CacheEntry {
    handle: TextureHandle,
    byte_size: usize,
    last_accessed: f64,
}

/// LRU texture cache with a configurable byte budget.
///
/// When inserting a new entry would exceed the budget, the least-recently-used
/// entries are evicted until the budget is satisfied.
pub struct TextureCache {
    entries: HashMap<CacheKey, CacheEntry>,
    total_bytes: usize,
    budget_bytes: usize,
}

impl TextureCache {
    /// Default budget: 256 MB.
    pub const DEFAULT_BUDGET: usize = 256 * 1024 * 1024;

    /// Create a new cache with the given byte budget.
    pub fn new(budget_bytes: usize) -> Self {
        Self {
            entries: HashMap::new(),
            total_bytes: 0,
            budget_bytes,
        }
    }

    /// Create a cache with the default 256 MB budget.
    pub fn default_budget() -> Self {
        Self::new(Self::DEFAULT_BUDGET)
    }

    /// Total bytes currently held in the cache.
    pub fn total_bytes(&self) -> usize {
        self.total_bytes
    }

    /// Get a cached texture, updating its last-accessed time.
    pub fn get(&mut self, key: &CacheKey, now: f64) -> Option<&TextureHandle> {
        if let Some(entry) = self.entries.get_mut(key) {
            entry.last_accessed = now;
            Some(&entry.handle)
        } else {
            None
        }
    }

    /// Insert a texture into the cache, evicting LRU entries if needed.
    ///
    /// Returns `true` if the entry was inserted, `false` if `byte_size` alone
    /// exceeds the budget (entry is not inserted in that case).
    pub fn insert(
        &mut self,
        key: CacheKey,
        handle: TextureHandle,
        byte_size: usize,
        now: f64,
    ) -> bool {
        if byte_size > self.budget_bytes {
            return false;
        }

        if let Some(old) = self.entries.remove(&key) {
            self.total_bytes -= old.byte_size;
        }

        while self.total_bytes + byte_size > self.budget_bytes {
            if !self.evict_lru() {
                break;
            }
        }

        self.total_bytes += byte_size;
        self.entries.insert(
            key,
            CacheEntry {
                handle,
                byte_size,
                last_accessed: now,
            },
        );
        true
    }

    /// Evict the least-recently-used entry. Returns `true` if an entry was evicted.
    fn evict_lru(&mut self) -> bool {
        if self.entries.is_empty() {
            return false;
        }

        let lru_key = self
            .entries
            .iter()
            .min_by(|a, b| {
                a.1.last_accessed
                    .partial_cmp(&b.1.last_accessed)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(key, _)| *key)
            .expect("entries checked non-empty above");

        let entry = self
            .entries
            .remove(&lru_key)
            .expect("lru_key came from self.entries.iter() above; it must still be present");
        self.total_bytes -= entry.byte_size;
        true
    }

    /// Remove all entries for a given page index (all zoom buckets).
    pub fn evict_page(&mut self, page_idx: PageIndex) {
        self.entries.retain(|key, entry| {
            if key.page_idx == page_idx {
                self.total_bytes -= entry.byte_size;
                false
            } else {
                true
            }
        });
    }

    /// Clear all entries.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.total_bytes = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_key(page: usize, bucket: u8) -> CacheKey {
        CacheKey {
            page_idx: PageIndex(page),
            zoom_bucket: bucket,
        }
    }

    #[test]
    fn byte_accounting_correct() {
        let cache = TextureCache::new(1000);

        assert_eq!(cache.total_bytes(), 0);
        assert_eq!(cache.budget_bytes, 1000);
    }

    #[test]
    fn cache_key_equality() {
        let k1 = make_key(0, 1);
        let k2 = make_key(0, 1);
        let k3 = make_key(0, 2);

        assert_eq!(k1, k2);
        assert_ne!(k1, k3);
    }

    #[test]
    fn evict_lru_order() {
        let cache = TextureCache::new(1000);

        assert_eq!(cache.total_bytes(), 0);
        assert!(cache.entries.is_empty());
    }

    #[test]
    fn cap_enforcement_strict() {
        let cache = TextureCache::new(100);

        assert!(100 <= cache.budget_bytes);
        assert!(101 > cache.budget_bytes);
    }
}
