//! Cache invariant tests for viewkai's LRU texture cache.

use egui::{ColorImage, Context, TextureOptions};
use egui_kittest::Harness;
use std::sync::{Mutex, OnceLock};
use viewkai::cache::{CacheKey, TextureCache};
use viewkai_core::page::PageIndex;

fn test_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn make_key(page: usize, bucket: u8) -> CacheKey {
    CacheKey {
        page_idx: PageIndex(page),
        zoom_bucket: bucket,
    }
}

fn make_texture(ctx: &Context, name: &str) -> egui::TextureHandle {
    let image = ColorImage::from_rgba_unmultiplied([1, 1], &[255, 255, 255, 255]);
    ctx.load_texture(name.to_owned(), image, TextureOptions::LINEAR)
}

#[test]
fn evict_lru_removes_exactly_one_entry() {
    let _guard = test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);

    let mut harness = Harness::new_ui(|_ui| {});
    harness.run();
    let ctx = harness.ctx.clone();

    let mut cache = TextureCache::new(300);
    assert!(cache.insert(make_key(0, 0), make_texture(&ctx, "lru-0"), 100, 0.0));
    assert!(cache.insert(make_key(1, 0), make_texture(&ctx, "lru-1"), 100, 1.0));
    assert!(cache.insert(make_key(2, 0), make_texture(&ctx, "lru-2"), 100, 2.0));

    assert!(cache.insert(make_key(3, 0), make_texture(&ctx, "lru-3"), 100, 3.0));

    assert_eq!(cache.total_bytes(), 300);
    assert!(cache.get(&make_key(0, 0), 4.0).is_none());
    assert!(cache.get(&make_key(1, 0), 4.0).is_some());
    assert!(cache.get(&make_key(2, 0), 4.0).is_some());
    assert!(cache.get(&make_key(3, 0), 4.0).is_some());
}

#[test]
fn evict_page_removes_all_zoom_buckets_for_page() {
    let _guard = test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);

    let mut harness = Harness::new_ui(|_ui| {});
    harness.run();
    let ctx = harness.ctx.clone();

    let mut cache = TextureCache::new(1_000);
    assert!(cache.insert(make_key(0, 0), make_texture(&ctx, "page-0-0"), 100, 0.0));
    assert!(cache.insert(make_key(0, 1), make_texture(&ctx, "page-0-1"), 100, 1.0));
    assert!(cache.insert(make_key(1, 0), make_texture(&ctx, "page-1-0"), 100, 2.0));

    cache.evict_page(PageIndex(0));

    assert_eq!(cache.total_bytes(), 100);
    assert!(cache.get(&make_key(0, 0), 3.0).is_none());
    assert!(cache.get(&make_key(0, 1), 3.0).is_none());
    assert!(cache.get(&make_key(1, 0), 3.0).is_some());
}

#[test]
fn insert_cannot_exceed_budget() {
    let _guard = test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);

    let mut harness = Harness::new_ui(|_ui| {});
    harness.run();
    let ctx = harness.ctx.clone();

    let mut cache = TextureCache::new(100);
    assert!(cache.insert(make_key(0, 0), make_texture(&ctx, "budget-0"), 60, 0.0));
    assert!(cache.insert(make_key(1, 0), make_texture(&ctx, "budget-1"), 90, 1.0));

    assert!(cache.total_bytes() <= 100);
    assert!(cache.get(&make_key(0, 0), 2.0).is_none());
    assert!(cache.get(&make_key(1, 0), 2.0).is_some());
}
