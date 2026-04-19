# viewkai v0.0.4 Baseline Metrics

Captured as part of Plan 01.75 Phase 0 on 2026-04-19.
These files record the state of the codebase **before** any architecture changes.

---

## Files

### `v0.0.4-tokei.json`
Workspace-level lines-of-code summary (Rust, TOML, etc.) for all files under `crates/`.

**Regen command:**
```sh
tokei --output json crates/ > .sisyphus/metrics/v0.0.4-tokei.json
```

**Key metric:** `jq '.Rust.code' .sisyphus/metrics/v0.0.4-tokei.json`

---

### `v0.0.4-tokei-files.json`
Per-file lines-of-code breakdown for all files under `crates/`.

**Regen command:**
```sh
tokei --files --output json crates/ > .sisyphus/metrics/v0.0.4-tokei-files.json
```

---

### `v0.0.4-clippy-pedantic.log`
Full output of `cargo clippy` with `-W clippy::pedantic` across the entire workspace.
Baseline warning count: **83 `^warning:` lines**.

**Regen command:**
```sh
cargo clippy --workspace --all-targets -- -W clippy::pedantic 2>&1 | tee .sisyphus/metrics/v0.0.4-clippy-pedantic.log
```

**Count warnings:**
```sh
grep -c '^warning:' .sisyphus/metrics/v0.0.4-clippy-pedantic.log
```

---

### `v0.0.4-tests-list.log`
Full output of `cargo test --workspace -- --list`, listing all test names.
Baseline test count: **37 tests** (`: test$` lines).

**Regen command:**
```sh
cargo test --workspace -- --list 2>&1 | tee .sisyphus/metrics/v0.0.4-tests-list.log
```

**Count tests:**
```sh
grep -c ': test$' .sisyphus/metrics/v0.0.4-tests-list.log
```

---

### `v0.0.4-ci-jobs.json`
Sorted list of CI job names from the most recent `ci.yml` run on `main`.
Baseline: 7 jobs — `build-wasm`, `clippy`, `fmt`, `purity-core`, `purity-engine`, `purity-viewkai`, `test`.

**Regen command:**
```sh
gh run view "$(gh run list --workflow=ci.yml --branch=main --limit=1 --json databaseId --jq '.[0].databaseId')" \
  --json jobs --jq '[.jobs[].name] | sort' > .sisyphus/metrics/v0.0.4-ci-jobs.json
```

---

### `v0.0.4-docs-list.txt`
Sorted list of documentation files found under `docs/`, `CONTRIBUTING.md`, and `README.md`.

**Regen command:**
```sh
find docs/ CONTRIBUTING.md README.md -type f 2>/dev/null | sort > .sisyphus/metrics/v0.0.4-docs-list.txt
```

---

## Summary Table

| Metric                  | v0.0.4 Baseline |
|-------------------------|-----------------|
| Rust LOC (code)         | 1865            |
| Clippy pedantic warnings| 83              |
| Test count              | 37              |
| CI jobs                 | 7               |
| Docs files              | 9               |
