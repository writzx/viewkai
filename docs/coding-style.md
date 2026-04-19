# Coding Style

Reference: [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)

## Naming (C-CASE, C-CONV)
- Types: `PascalCase`; functions/methods: `snake_case`; constants: `UPPER_SNAKE_CASE`
- Predicate methods return `bool` and are named `is_foo` or `has_foo` (C-CONV)
- Getters omit the `get_` prefix; setters use `set_foo` (C-CONV)
- Iterators are named `iter()` / `iter_mut()` / `into_iter()` (C-ITER)
- No module-name repetition in exported types — `cache::Cache` not `cache::CacheCache` (allowed in [workspace.lints]: see `module_name_repetitions = "allow"`)

## Error Handling
See [docs/error-handling.md](error-handling.md) for the full policy.
Short form: library crates return `Result<T, E>` with a `thiserror`-derived `E`; never `Result<T, String>`.

## Module Organization
- Prefer files of 50–300 LOC
- Split when a file exceeds 400 LOC AND has multiple cohesive sub-concerns (not just one large impl block)
- New crate vs new module: new crate only when the dep graph requires a hard boundary (e.g., pdfium-render must not leak into viewkai)

## Doc Comments
- All `pub` items in library crates (`viewkai`, `viewkai-core`, `viewkai-engine`) require `///` doc comments
- Minimum: one sentence describing intent
- Non-trivial public functions should include `# Examples` with a doctest

## Unsafe Code
- Forbidden in `viewkai`, `viewkai-core`, `viewkai-app`, and `viewkai-web`
- Permitted in `viewkai-engine` only for self-referential struct patterns (pdfium document caching)
- Every `unsafe` block requires a `// SAFETY:` comment of ≥ 10 lines covering: aliasing rules, lifetime assumptions, drop order, and absence of escaped borrows
- `unsafe` in `#[cfg(test)]` requires the same treatment

## Panics
- `.unwrap()` and `expect()` are banned in library crate non-test code
- `.expect("reason")` is permitted in `#[cfg(test)]` only (see error-handling.md §3)
- Index via `.get(i).expect(...)` not `arr[i]` in production paths

## Formatting
- `cargo fmt` is enforced by CI; never suppress it with `#[rustfmt::skip]` unless the alternative is genuinely less readable
- Line width: 100 columns (rustfmt default)
