# Error Handling

Reference: [Rust API Guidelines C-GOOD-ERR](https://rust-lang.github.io/api-guidelines/interoperability.html#error-types-are-meaningful-and-well-behaved-c-good-err)

## §1 — Library crates use typed enums, never String

Library crates (`viewkai`, `viewkai-core`, `viewkai-engine`) return:
```rust
Result<T, SomeError>
```
where `SomeError` is a crate-local `enum` derived with `#[derive(Debug, thiserror::Error)]`.

Returning `Result<T, String>` or `Result<T, Box<dyn Error>>` is a code smell; the compiler cannot help callers match on error cases.

## §2 — Error conversion uses `#[from]`, string formatting is for display only

Use `#[from]` to implement `From<SourceError>` automatically:
```rust
#[derive(thiserror::Error, Debug)]
enum MyError {
    #[error("engine failed: {0}")]
    Engine(#[from] EngineError),  // From<EngineError> generated automatically
}
```

`format!("{e}")` or `.to_string()` on an error is acceptable ONLY when storing a string in a catch-all variant (e.g., `Pdfium { message: String }`) or in user-facing display. Never use it to convert between error types.

## §3 — `.unwrap()` and `.expect()` rules

| Context | `.unwrap()` | `.expect("reason")` |
|---|---|---|
| Library production code | ❌ banned | ❌ banned |
| `#[cfg(test)]` or `tests/` | ✅ allowed | ✅ preferred |
| `viewkai-app` / `viewkai-web` (binary/demo crates) | ⚠️ discouraged, prefer `?` | ✅ with clear message |

Use `.expect("invariant: explanation")` rather than `.unwrap()` to document WHY the None/Err case is impossible.

## §4 — Preserving pdfium error structure (viewkai-engine)

`pdfium_render::error::PdfiumError` is an internal type that must NOT appear in `viewkai-engine`'s public API (Invariant 3). The engine maps it:
- Known variants → structured `EngineError` variants (e.g., `PageIndexOutOfBounds { index, count }`)
- Unknown variants → `EngineError::Pdfium { message: e.to_string() }`

The mapping lives in `crates/viewkai-engine/src/error.rs`. Do NOT use `#[from] PdfiumError` as an attribute because `thiserror` 2.x exposes the source type via `.source()` downcasting, which would leak it through the public API.

## §5 — `?` operator and error propagation

Use `?` freely within a function. When `?` would produce an unhelpful message, construct the error variant directly:
```rust
// Good: caller has the index and count handy
return Err(EngineError::PageIndexOutOfBounds { index: idx as u32, count: page_count as u32 });

// Avoid: loses structured data
return Err(e)?;  // only if EngineError::from(e) produces a good variant
```
