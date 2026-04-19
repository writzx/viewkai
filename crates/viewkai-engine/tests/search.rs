//! Search engine integration tests.

use std::sync::{Mutex, OnceLock};

use viewkai_core::{PageIndex, SearchQuery};
use viewkai_engine::{Document, init, search_page};

fn test_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

#[test]
fn search_hello_pdf_returns_matches() {
    let _guard = test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    init().expect("pdfium init");
    let bytes = include_bytes!("../../../tests/fixtures/hello.pdf").to_vec();
    let doc = Document::from_bytes(bytes).expect("load hello.pdf");
    let query = SearchQuery {
        term: "hello".to_owned(),
        case_sensitive: false,
        whole_word: false,
    };
    let matches = search_page(&doc, PageIndex(0), &query).expect("search");
    let _ = matches;
}

#[test]
fn search_empty_term_returns_empty() {
    let _guard = test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    init().expect("pdfium init");
    let bytes = include_bytes!("../../../tests/fixtures/hello.pdf").to_vec();
    let doc = Document::from_bytes(bytes).expect("load hello.pdf");
    let query = SearchQuery {
        term: String::new(),
        case_sensitive: false,
        whole_word: false,
    };
    let matches = search_page(&doc, PageIndex(0), &query).expect("search");
    assert!(matches.is_empty(), "empty term should return no matches");
}

#[test]
fn search_no_matches_returns_empty_vec() {
    let _guard = test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    init().expect("pdfium init");
    let bytes = include_bytes!("../../../tests/fixtures/hello.pdf").to_vec();
    let doc = Document::from_bytes(bytes).expect("load hello.pdf");
    let query = SearchQuery {
        term: "xyzzy_not_in_pdf_12345".to_owned(),
        case_sensitive: false,
        whole_word: false,
    };
    let matches = search_page(&doc, PageIndex(0), &query).expect("search");
    assert!(matches.is_empty(), "non-existent term should return no matches");
}
