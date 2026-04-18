use criterion::{criterion_group, criterion_main, Criterion};
use viewkai_core::page::PageIndex;

fn bench_parse_500_page_doc(c: &mut Criterion) {
    viewkai_engine::init().expect("Failed to initialize PDFium");
    let bytes = include_bytes!("../../../tests/fixtures/500page.pdf").to_vec();

    c.bench_function("parse_500_page_doc", |b| {
        b.iter(|| {
            let doc = viewkai_engine::Document::from_bytes(bytes.clone())
                .expect("should open 500page.pdf");
            criterion::black_box(doc.page_count())
        })
    });
}

fn bench_rasterize_page_at_150dpi(c: &mut Criterion) {
    viewkai_engine::init().expect("Failed to initialize PDFium");
    let bytes = include_bytes!("../../../tests/fixtures/500page.pdf").to_vec();
    let doc = viewkai_engine::Document::from_bytes(bytes).expect("should open 500page.pdf");

    c.bench_function("rasterize_page_at_150dpi", |b| {
        b.iter(|| {
            let raw = viewkai_engine::render_page(&doc, PageIndex(0), 150)
                .expect("should render page");
            criterion::black_box(raw.pixels.len())
        })
    });
}

criterion_group!(benches, bench_parse_500_page_doc, bench_rasterize_page_at_150dpi);
criterion_main!(benches);
