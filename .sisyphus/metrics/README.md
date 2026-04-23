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

| Metric                  | v0.0.4 Baseline | v0.1.0 Baseline |
|-------------------------|-----------------|-----------------|
| Rust LOC (code)         | 1865            | 4754            |
| Clippy pedantic warnings| 83              | 0               |
| Test count              | 37              | 74              |
| CI jobs                 | 7               | 16              |
| Docs files              | 9               | 15              |

---

## v0.1.0 Baseline Metrics

Captured as part of Plan 02.5 Phase 0.6 on 2026-04-20.
These files record the state of the codebase after Plan 02 (`v0.1.0`, commit `01bbe07`) shipped,
with Phase 0.1-0.4 CI fixes applied (fmt, clippy, machete, wasm-bindgen JsCast import).

### Files

#### `v0.1.0-tokei.json`
Workspace-level lines-of-code summary for all files under `crates/`.
Key metric: `jq '.Rust.code' .sisyphus/metrics/v0.1.0-tokei.json` → **4754 lines**

**Regen command:**
```sh
tokei --output json crates/ > .sisyphus/metrics/v0.1.0-tokei.json
```

#### `v0.1.0-tokei-files.json`
Per-file lines-of-code breakdown for all files under `crates/`.

**Regen command:**
```sh
tokei --files --output json crates/ > .sisyphus/metrics/v0.1.0-tokei-files.json
```

#### `v0.1.0-clippy-pedantic.log`
Full output of `cargo clippy` with `-W clippy::pedantic` across the entire workspace.
Baseline warning count: **0 `^warning:` lines** (all pedantic warnings cleaned up).

**Regen command:**
```sh
cargo clippy --workspace --all-targets -- -W clippy::pedantic 2>&1 | tee .sisyphus/metrics/v0.1.0-clippy-pedantic.log
```

#### `v0.1.0-tests-list.log`
Full output of `cargo test --workspace -- --list`, listing all test names.
Baseline test count: **74 tests** (`: test$` lines).

**Regen command:**
```sh
cargo test --workspace -- --list 2>&1 | tee .sisyphus/metrics/v0.1.0-tests-list.log
```

#### `v0.1.0-ci-jobs.json`
Sorted list of CI job names from the most recent `ci.yml` run on `main`.
Baseline: **16 jobs** — `build-native`, `build-wasm`, `clippy`, `deny`, `docs`, `fmt`, `hack`, `machete`, `msrv`, `purity-app`, `purity-core`, `purity-engine`, `purity-plugins`, `purity-viewkai`, `purity-web`, `test`.

**Regen command:**
```sh
gh run view "$(gh run list --workflow=ci.yml --branch=main --limit=1 --json databaseId --jq '.[0].databaseId')" \
  --json jobs --jq '[.jobs[].name] | sort' > .sisyphus/metrics/v0.1.0-ci-jobs.json
```

#### `v0.1.0-docs-list.txt`
Sorted list of documentation files found under `docs/`, `CONTRIBUTING.md`, and `README.md`.
Baseline: **15 files**.

**Regen command:**
```sh
find docs/ CONTRIBUTING.md README.md -type f 2>/dev/null | sort > .sisyphus/metrics/v0.1.0-docs-list.txt
```

---

## v0.2.0 Baseline Metrics

Captured as part of Plan 03.25 Phase 0.0 on 2026-04-23.
These files record the state of the codebase at `v0.2.0` (all 6 crates), before any UI-polish changes.

### Files

#### `v0.2.0-tokei.json`
Workspace-level lines-of-code summary for all files under `crates/`.
Key metric: `python3 -c "import json; d=json.load(open('.sisyphus/metrics/v0.2.0-tokei.json')); print(d['Rust']['code'])"` → **9035 lines**

**Regen command:**
```sh
tokei --output json crates/ > .sisyphus/metrics/v0.2.0-tokei.json
```

#### `v0.2.0-tokei-files.json`
Per-file lines-of-code breakdown for all files under `crates/`.

**Regen command:**
```sh
tokei --files --output json crates/ > .sisyphus/metrics/v0.2.0-tokei-files.json
```

#### `v0.2.0-clippy-pedantic.log`
Full output of `cargo clippy` with `-W clippy::pedantic` across the entire workspace.
Baseline warning count: **27 `^warning:` lines**.

**Regen command:**
```sh
cargo clippy --workspace --all-targets -- -W clippy::pedantic 2>&1 | tee .sisyphus/metrics/v0.2.0-clippy-pedantic.log
```

**Count warnings:**
```sh
grep -c '^warning:' .sisyphus/metrics/v0.2.0-clippy-pedantic.log
```

#### `v0.2.0-tests-list.log`
Full output of `cargo test --workspace -- --list`, listing all test names.
Baseline test count: **148 tests** (`: test$` lines).

**Regen command:**
```sh
cargo test --workspace -- --list 2>&1 | tee .sisyphus/metrics/v0.2.0-tests-list.log
```

**Count tests:**
```sh
grep -c ': test$' .sisyphus/metrics/v0.2.0-tests-list.log
```

#### `v0.2.0-ci-jobs.json`
CI jobs list — placeholder (gh CLI not available in capture environment).
Content: `["ci-jobs-not-available"]`

**Regen command (when gh CLI available):**
```sh
gh run view "$(gh run list --workflow=ci.yml --branch=main --limit=1 --json databaseId --jq '.[0].databaseId')" \
  --json jobs --jq '[.jobs[].name] | sort' > .sisyphus/metrics/v0.2.0-ci-jobs.json
```

#### `v0.2.0-docs-list.txt`
Sorted list of documentation files found under `docs/`, `CONTRIBUTING.md`, and `README.md`.
Baseline: **22 files**.

**Regen command:**
```sh
find docs/ CONTRIBUTING.md README.md -type f | sort > .sisyphus/metrics/v0.2.0-docs-list.txt
```

---

## Summary Table

| Metric                   | v0.0.4 | v0.1.0 | v0.2.0 |
|--------------------------|--------|--------|--------|
| Rust LOC (code)          | 1865   | 4754   | 9035   |
| Clippy pedantic warnings | 83     | 0      | 27     |
| Test count               | 37     | 74     | 148    |
| CI jobs                  | 7      | 16     | N/A    |
| Docs files               | 9      | 15     | 22     |
