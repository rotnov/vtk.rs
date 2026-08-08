# `cargo xtask ledger-check` Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement `cargo xtask ledger-check`, the four-assertion CI gate over
`docs/test-mapping.csv` described in `docs/decisions/0003-upstream-sync-strategy.md` and required
by [issue #41](https://github.com/rotnov/vtk.rs/issues/41). It is dependency-order Step 3 in
`docs/superpowers/specs/2026-08-06-autonomous-operation-design.md` and the hard prerequisite for
adding required status checks to branch protection.

**Architecture:** A new `xtask` binary crate in the `rust/` workspace, invoked as
`cargo xtask ledger-check` via a `[alias]` in `rust/.cargo/config.toml`. Five small modules, each
pure logic with its own unit tests, wired together by a thin `main.rs`: a CSV ledger parser, a
hand-rolled CMake test-macro-call parser (no CMake execution — text only), a git-backed
reference-tree lookup, a crate-contains-code detector, and the four assertion functions. The tool
shells out to the system `git` binary for blob/tree queries instead of a git library, and uses the
`csv` crate for RFC4180-correct parsing instead of hand-rolled splitting.

**Tech Stack:** Rust stable (matches `rust/rust-toolchain.toml`), `csv` crate for ledger parsing,
system `git` CLI shelled out via `std::process::Command` for reference-tree queries.

## Global Constraints

- **Scope is exactly issue #41: the four assertions.** `test-mapping-report` and `upstream-diff`
  are named in `AGENTS.md` § Commands as future `cargo xtask` subcommands but neither is this
  plan's job — `upstream-diff` has its own issue
  ([#44](https://github.com/rotnov/vtk.rs/issues/44)); `test-mapping-report` has no filed issue at
  all. Do not implement either. The `xtask` binary's CLI only needs to recognize `ledger-check`
  today; an unrecognized subcommand should print a usage message and exit 2, not panic.

- **The four assertions, verbatim from `docs/decisions/0003-upstream-sync-strategy.md`:**
  - **exists** — every `original_path` in `docs/test-mapping.csv` is present in the reference tree.
  - **complete** — for every module with at least one ledger row, every test registered in that
    module's `Testing/*/CMakeLists.txt` has a row. Scoped to started modules deliberately —
    unscoped it fails on all untouched tests on day one.
  - **fresh** — every row's `original_sha` still matches the blob SHA of its `original_path`.
  - **parity** — every crate that contains any code has at least one ledger row with
    `status=ported` for its module.

- **Ledger schema, verbatim from `AGENTS.md` § The test-mapping ledger** (file:
  `docs/test-mapping.csv`, header already committed):
  `original_path,original_test,original_sha,rust_path,rust_test,category,status,notes`. One row per
  original test function. `status` is one of `deferred`, `spec`, `ported`, `skipped`. `notes` is
  "required for `skipped` and `deferred`, free text otherwise" — free text is exploited below for
  the unresolved-variable clearing convention.

- **Module identity (for *complete*):** derived from a ledger row's `original_path` by truncating
  at `/Testing/` — e.g. `Common/Core/Testing/Cxx/TestArrayAPI.cxx` → module `Common/Core`. A module
  is "started" iff at least one ledger row's computed module equals it.

- **Crate identity (for *parity*):** derived from a ledger row's `rust_path` by prefix match
  against `rust/crates/<crate-dir-name>/` — e.g. `rust/crates/vtk-common-core/src/array/api.rs`
  belongs to crate dir `vtk-common-core`. No VTK-module-name-to-crate-name mapping table; parity
  never needs to know a crate's VTK module, only whether *some* ported row's `rust_path` falls
  under it.

- **"Contains any code" (for *parity*), a mechanical, textual definition:** a crate's `src/`
  directory "contains code" iff at least one `.rs` file under it has a line that, after trimming
  whitespace, is non-empty and does not start with `//`. Every one of the 7 existing crates today
  is exactly a single `//!` doc-comment line in `src/lib.rs` (verified: `find rust/crates -name
  lib.rs -exec cat {} \;` shows one `//!` line each, nothing else) — this definition correctly
  reports "no code" for all of them today and flips to "has code" the moment real Rust syntax
  lands, which is exactly when porting starts. Known limitation, accepted: a multi-line `/* */`
  block comment whose continuation lines don't start with `//` would be mis-detected as code; this
  codebase's style doesn't use block comments, so this is not expected to bite.

- **CMake test-macro-call syntax — two token forms, confirmed by reading
  `CMake/vtkModuleTesting.cmake` `_vtk_test_parse_args`/`_vtk_test_parse_name`
  (lines 192-282) directly, not inferred:**
  - **Bare form:** `Name.ext` → test name is `Name` (extension stripped).
  - **Comma form:** `left,right` → if `left` ends with `.ext`, this is a per-test-options entry
    (`Name.ext,OPTION1,OPTION2`) and the test name is `left` with `.ext` stripped. If `left` does
    **not** end with `.ext`, this is a custom-name-plus-path entry (as generated by
    `Common/Core/Testing/Cxx/CMakeLists.txt`'s `add_data_array_test` function, e.g.
    `TestDataArrayAPI_vtkCharArray,${CMAKE_CURRENT_BINARY_DIR}/TestDataArrayAPI_vtkCharArray.cxx`)
    and the test name is `left` verbatim, unstripped.
  - A bare token that exactly matches one of the macro's known block-level option keywords (see
    `CXX_OPTIONS`/`PYTHON_OPTIONS` below) is a flag for the whole call, not a test.
  - `vtk_add_test_cxx` and `vtk_add_test_python` share this exact parsing shape (confirmed:
    `vtk_add_test_python` at line 778 calls the same `_vtk_test_parse_args`/`_vtk_test_parse_name`
    functions, parameterized by extension `"py"` and its own options list) — one parser function,
    parameterized by `(macro_name, ext, known_options)`, handles both. `CXX_OPTIONS = ["NO_DATA",
    "NO_VALID", "NO_OUTPUT", "TIGHT_VALID", "LOOSE_VALID", "LEGACY_VALID",
    "WEBGPU_GRAPHICS_BACKEND"]`. `PYTHON_OPTIONS = ["NO_DATA", "NO_VALID", "NO_OUTPUT", "NO_RT",
    "DIRECT_DATA", "JUST_VALID", "LEGACY_VALID", "TIGHT_VALID", "LOOSE_VALID"]`.
  - Per `AGENTS.md`'s literal wording (`Testing/*/CMakeLists.txt`), `complete` looks for both
    `Testing/Cxx/CMakeLists.txt` and `Testing/Python/CMakeLists.txt` under a started module. Any
    *other* `Testing/<Something>/CMakeLists.txt` the parser finds under a started module (e.g.
    `Testing/MPI`) is reported as a non-fatal "unparsed test directory, verify manually" note, not
    a hard violation — out of scope for v1, and no Phase 1 module is expected to have one.

- **Variable-splicing is permanently out of scope for resolution — flagged, never resolved.** At
  least one real case (`Common/Core/Testing/Cxx/CMakeLists.txt`'s `${data_array_tests}`, populated
  via a `foreach` loop, a custom `function()`, and build-time `configure_file()` templating) cannot
  be resolved by any text-based parser without embedding a CMake interpreter — confirmed by reading
  the generator function in full. The parser never attempts partial resolution (not even for
  trivial `set(var a b c)` cases) — uniform behavior is simpler to implement and verify than a
  two-tier heuristic, and the cost is small: whenever a `${identifier}` token appears in a macro
  call's argument list, it is recorded as **unresolved**, full stop.
  - **Clearing convention (new, introduced by this plan):** an unresolved variable
    `${data_array_tests}` in module `Common/Core` is cleared by *complete* iff at least one ledger
    row whose computed module is `Common/Core` has `notes` containing the substring
    `generated:data_array_tests` (any row, any status — the marker records that a human already
    manually enumerated that variable's generated tests into individual rows, which is required
    work regardless of what this checker can see). No ledger schema change — `notes` is already
    free text. Document this convention in `AGENTS.md` § The test-mapping ledger (Task 7).
  - An unresolved variable with no such row is a **complete** violation naming the file, the
    variable, and the exact marker text a human needs to add once satisfied.

- **Positive-control requirement, verbatim from issue #41's "trap to avoid":** "All four assertions
  pass vacuously on today's empty ledger... Each assertion needs its own positive-control test that
  makes it actually fail, plus unit tests with fixtures for the `Testing/*/CMakeLists.txt` parser."
  Every assertion's test module must include at least one fixture that is deliberately wrong and
  asserts a non-empty violation list — a green pass on a clean fixture alone does not satisfy this.

- **No I/O in assertion logic.** `check_exists`/`check_fresh` take an injected lookup function
  (`impl Fn(&str) -> bool` / `impl Fn(&str) -> Option<String>`) rather than touching the filesystem
  or shelling to git directly — this is what makes the positive-control fixtures above unit-testable
  without a real git repo. The real git-backed implementations live in `reference_tree.rs` and are
  wired in only by `main.rs`.

- **`xtask` is dev-only tooling, not shipped port code — the `vtk-*` crate dependency policy does
  not bind it.** `AGENTS.md`'s "a crate's `Cargo.toml` dependencies must mirror that module's
  `DEPENDS` graph" and "don't introduce a dependency VTK's own module graph doesn't have" rules
  (§ Dependency graph) govern the ported `vtk-*` crates' correspondence to VTK's own module graph;
  they say nothing about build/CI tooling, and `xtask` ships nothing a downstream consumer links
  against. Task 1 depends on the `csv` crate (v1) for RFC4180-correct parsing of
  `docs/test-mapping.csv` (needed because `notes` is free text and may itself contain commas inside
  quotes, e.g. `"no Rust analogue, see ADR 0001"`) — this is an explicit, deliberate exception to
  the repo's general minimal-dependency posture, scoped to `xtask` alone. Do not add `csv` (or any
  other new third-party dependency) to any `vtk-*` crate.

- **`original_test` is the bare CMake-call test name, not the registered CTest name.** `AGENTS.md`
  § The test-mapping ledger describes `original_test` as "the registered CTest name," which in
  VTK's actual CTest output carries an executable-name prefix (e.g.
  `vtkCommonCoreCxx-TestArrayAPI`). This plan's parser (Task 2) produces the bare name as it
  appears in the CMake macro call (`TestArrayAPI`), because deriving the prefixed CTest name would
  require resolving which test executable a `Testing/Cxx/CMakeLists.txt` belongs to — out of scope
  and unnecessary for *complete*, which only needs to match ledger rows against parsed call sites,
  both using the bare name consistently. Task 7 updates `AGENTS.md` to pin this definition
  explicitly so a future porter doesn't write the prefixed form and get spurious *complete*
  failures.

- **Writable paths for this plan:** `rust/` (new `xtask` crate, `Cargo.toml`, `.cargo/config.toml`),
  `.github/workflows/rust-checks.yml`, `AGENTS.md` (§ Commands, § The test-mapping ledger). No
  reference-tree path is ever modified — this tool only reads it (`AGENTS.md` § What is writable
  already forbids touching the reference tree).

---

### Task 1: Ledger CSV model and parser

**Files:**
- Create: `rust/xtask/Cargo.toml`
- Create: `rust/xtask/src/main.rs` (minimal stub in this task — Task 6 replaces it with full
  orchestration; every task from here through Task 5 adds one `mod` line to it so the crate
  compiles with each new file)
- Create: `rust/xtask/src/ledger.rs`
- Modify: `rust/Cargo.toml` (add `"xtask"` to `members`)

**Interfaces:**
- Produces: `pub struct LedgerRow { pub original_path: String, pub original_test: String,
  pub original_sha: String, pub rust_path: String, pub rust_test: String, pub category: String,
  pub status: String, pub notes: String }`
- Produces: `pub fn parse_ledger(csv_text: &str) -> Result<Vec<LedgerRow>, String>`
- Produces: `pub fn module_of(original_path: &str) -> Option<&str>` — returns the substring before
  `/Testing/`, or `None` if `original_path` contains no `/Testing/` segment.

- [ ] **Step 1: Create the `xtask` crate skeleton**

```bash
mkdir -p rust/xtask/src
```

Write `rust/xtask/Cargo.toml`:

```toml
[package]
name = "xtask"
version.workspace = true
edition.workspace = true
publish.workspace = true
description = "Dev-only CI tooling: cargo xtask ledger-check."

[dependencies]
csv = "1"
```

(`publish.workspace = true` matches the existing 7 crates' pattern in `rust/crates/*/Cargo.toml`,
inheriting `publish = false` from `[workspace.package]`.)

- [ ] **Step 1b: Create a minimal `main.rs` so the crate compiles**

A binary crate needs `src/main.rs` to exist before `cargo test -p xtask` will compile anything in
it, including `ledger.rs` below (an unreferenced `.rs` file is not part of the crate). Write
`rust/xtask/src/main.rs`:

```rust
mod ledger;

fn main() {
    eprintln!("cargo xtask: no subcommand implemented yet");
    std::process::exit(2);
}
```

- [ ] **Step 2: Add `xtask` to the workspace**

Modify `rust/Cargo.toml` — add `"xtask"` to the `members` list (keep the existing 7 entries,
append this one last since it isn't a `vtk-common-*` crate):

```toml
[workspace]
resolver = "3"
members = [
    "crates/vtk-common-core",
    "crates/vtk-common-math",
    "crates/vtk-common-system",
    "crates/vtk-common-transforms",
    "crates/vtk-common-misc",
    "crates/vtk-common-data-model",
    "crates/vtk-common-execution-model",
    "xtask",
]
```

- [ ] **Step 3: Write the failing tests for `parse_ledger` and `module_of`**

Write `rust/xtask/src/ledger.rs`:

```rust
use std::collections::HashMap;

pub struct LedgerRow {
    pub original_path: String,
    pub original_test: String,
    pub original_sha: String,
    pub rust_path: String,
    pub rust_test: String,
    pub category: String,
    pub status: String,
    pub notes: String,
}

pub fn parse_ledger(csv_text: &str) -> Result<Vec<LedgerRow>, String> {
    todo!()
}

pub fn module_of(original_path: &str) -> Option<&str> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEADER: &str =
        "original_path,original_test,original_sha,rust_path,rust_test,category,status,notes\n";

    #[test]
    fn empty_ledger_has_no_rows() {
        let rows = parse_ledger(HEADER).unwrap();
        assert!(rows.is_empty());
    }

    #[test]
    fn parses_a_single_row() {
        let csv = format!(
            "{HEADER}Common/Core/Testing/Cxx/TestArrayAPI.cxx,TestArrayAPI,abc123,\
             rust/crates/vtk-common-core/src/array/api.rs,array_api_roundtrip,1,ported,\n"
        );
        let rows = parse_ledger(&csv).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].original_test, "TestArrayAPI");
        assert_eq!(rows[0].status, "ported");
    }

    #[test]
    fn parses_multiple_rows_across_modules() {
        let csv = format!(
            "{HEADER}\
             Common/Core/Testing/Cxx/TestArrayAPI.cxx,TestArrayAPI,sha1,p1,t1,1,ported,\n\
             Common/Math/Testing/Cxx/TestMath.cxx,TestMath,sha2,p2,t2,1,deferred,phase 2\n"
        );
        let rows = parse_ledger(&csv).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[1].notes, "phase 2");
    }

    #[test]
    fn a_note_containing_a_comma_round_trips_when_quoted() {
        let csv = format!(
            "{HEADER}Common/Core/Testing/Cxx/TestFoo.cxx,TestFoo,sha,p,t,1,skipped,\
             \"no Rust analogue, see ADR 0001\"\n"
        );
        let rows = parse_ledger(&csv).unwrap();
        assert_eq!(rows[0].notes, "no Rust analogue, see ADR 0001");
    }

    #[test]
    fn malformed_row_is_an_error_not_a_panic() {
        let csv = format!("{HEADER}too,few,columns\n");
        assert!(parse_ledger(&csv).is_err());
    }

    #[test]
    fn module_of_strips_at_testing_segment() {
        assert_eq!(
            module_of("Common/Core/Testing/Cxx/TestArrayAPI.cxx"),
            Some("Common/Core")
        );
        assert_eq!(
            module_of("Common/DataModel/Testing/Python/TestFoo.py"),
            Some("Common/DataModel")
        );
    }

    #[test]
    fn module_of_returns_none_without_a_testing_segment() {
        assert_eq!(module_of("Common/Core/vtkObject.cxx"), None);
    }
}
```

- [ ] **Step 4: Run the tests to verify they fail**

```bash
cd rust && cargo test -p xtask ledger:: 2>&1 | tail -20
```

Expected: compile error or panic from the two `todo!()` bodies.

- [ ] **Step 5: Implement `parse_ledger` and `module_of`**

Replace the two `todo!()` bodies in `rust/xtask/src/ledger.rs`:

```rust
pub fn parse_ledger(csv_text: &str) -> Result<Vec<LedgerRow>, String> {
    let mut reader = csv::ReaderBuilder::new().from_reader(csv_text.as_bytes());
    let mut rows = Vec::new();
    for (i, result) in reader.records().enumerate() {
        let record = result.map_err(|e| format!("row {}: {e}", i + 1))?;
        if record.len() != 8 {
            return Err(format!(
                "row {}: expected 8 columns, found {}",
                i + 1,
                record.len()
            ));
        }
        rows.push(LedgerRow {
            original_path: record[0].to_string(),
            original_test: record[1].to_string(),
            original_sha: record[2].to_string(),
            rust_path: record[3].to_string(),
            rust_test: record[4].to_string(),
            category: record[5].to_string(),
            status: record[6].to_string(),
            notes: record[7].to_string(),
        });
    }
    Ok(rows)
}

pub fn module_of(original_path: &str) -> Option<&str> {
    original_path.split_once("/Testing/").map(|(module, _)| module)
}
```

Remove the now-unused `use std::collections::HashMap;` import at the top of the file.

- [ ] **Step 6: Run the tests to verify they pass**

```bash
cd rust && cargo test -p xtask ledger::
```

Expected: all 7 tests pass.

- [ ] **Step 7: Commit**

```bash
git add rust/Cargo.toml rust/xtask/Cargo.toml rust/xtask/src/main.rs rust/xtask/src/ledger.rs
git commit -m "xtask: ledger CSV parser and module_of helper"
```

---

### Task 2: CMake test-macro-call parser

**Files:**
- Create: `rust/xtask/src/cmake_parser.rs`
- Modify: `rust/xtask/src/main.rs` (add `mod cmake_parser;`)

**Interfaces:**
- Consumes: nothing from Task 1.
- Produces: `pub struct ParsedTest { pub name: String, pub raw_token: String }`
- Produces: `pub struct ParsedCMakeFile { pub tests: Vec<ParsedTest>, pub unresolved: Vec<String> }`
- Produces: `pub const CXX_OPTIONS: &[&str]`, `pub const PYTHON_OPTIONS: &[&str]` (see Global
  Constraints for exact values).
- Produces: `pub fn parse_test_macro_calls(text: &str, macro_name: &str, ext: &str, known_options:
  &[&str]) -> ParsedCMakeFile`

- [ ] **Step 1: Write the failing tests**

Write `rust/xtask/src/cmake_parser.rs`:

```rust
pub const CXX_OPTIONS: &[&str] = &[
    "NO_DATA",
    "NO_VALID",
    "NO_OUTPUT",
    "TIGHT_VALID",
    "LOOSE_VALID",
    "LEGACY_VALID",
    "WEBGPU_GRAPHICS_BACKEND",
];

pub const PYTHON_OPTIONS: &[&str] = &[
    "NO_DATA",
    "NO_VALID",
    "NO_OUTPUT",
    "NO_RT",
    "DIRECT_DATA",
    "JUST_VALID",
    "LEGACY_VALID",
    "TIGHT_VALID",
    "LOOSE_VALID",
];

#[derive(Debug, PartialEq)]
pub struct ParsedTest {
    pub name: String,
    pub raw_token: String,
}

#[derive(Debug, Default)]
pub struct ParsedCMakeFile {
    pub tests: Vec<ParsedTest>,
    pub unresolved: Vec<String>,
}

pub fn parse_test_macro_calls(
    text: &str,
    macro_name: &str,
    ext: &str,
    known_options: &[&str],
) -> ParsedCMakeFile {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(result: &ParsedCMakeFile) -> Vec<&str> {
        result.tests.iter().map(|t| t.name.as_str()).collect()
    }

    #[test]
    fn bare_form_strips_extension() {
        let text = "vtk_add_test_cxx(vtkCommonCoreCxx tests\n  TestArrayAPI.cxx\n  )";
        let result = parse_test_macro_calls(text, "vtk_add_test_cxx", "cxx", CXX_OPTIONS);
        assert_eq!(names(&result), vec!["TestArrayAPI"]);
        assert!(result.unresolved.is_empty());
    }

    #[test]
    fn per_test_comma_options_form_strips_extension() {
        let text = "vtk_add_test_cxx(vtkCommonCoreCxx tests\n  TestArrayAPI.cxx,NO_VALID\n  )";
        let result = parse_test_macro_calls(text, "vtk_add_test_cxx", "cxx", CXX_OPTIONS);
        assert_eq!(names(&result), vec!["TestArrayAPI"]);
    }

    #[test]
    fn custom_name_and_path_comma_form_keeps_name_unstripped() {
        // Mirrors Common/Core/Testing/Cxx/CMakeLists.txt's add_data_array_test-generated entries.
        let text = "vtk_add_test_cxx(vtkCommonCoreCxx tests\n  \
            TestDataArrayAPI_vtkCharArray,${CMAKE_CURRENT_BINARY_DIR}/TestDataArrayAPI_vtkCharArray.cxx\n  )";
        let result = parse_test_macro_calls(text, "vtk_add_test_cxx", "cxx", CXX_OPTIONS);
        assert_eq!(names(&result), vec!["TestDataArrayAPI_vtkCharArray"]);
    }

    #[test]
    fn block_level_bare_option_is_not_a_test() {
        let text = "vtk_add_test_cxx(vtkCommonCoreCxx tests\n  NO_VALID\n  TestFoo.cxx\n  )";
        let result = parse_test_macro_calls(text, "vtk_add_test_cxx", "cxx", CXX_OPTIONS);
        assert_eq!(names(&result), vec!["TestFoo"]);
    }

    #[test]
    fn unresolved_variable_token_is_flagged_not_silently_dropped() {
        let text = "vtk_add_test_cxx(vtkCommonCoreCxx tests\n  ${data_array_tests}\n  )";
        let result = parse_test_macro_calls(text, "vtk_add_test_cxx", "cxx", CXX_OPTIONS);
        assert!(result.tests.is_empty());
        assert_eq!(result.unresolved, vec!["data_array_tests"]);
    }

    #[test]
    fn mixed_literals_and_variable_in_one_call() {
        // The actual shape of Common/Core/Testing/Cxx/CMakeLists.txt's main vtk_add_test_cxx call:
        // literal test names followed by a spliced-in variable.
        let text =
            "vtk_add_test_cxx(vtkCommonCoreCxx tests\n  TestArrayAPI.cxx\n  ${data_array_tests}\n  )";
        let result = parse_test_macro_calls(text, "vtk_add_test_cxx", "cxx", CXX_OPTIONS);
        assert_eq!(names(&result), vec!["TestArrayAPI"]);
        assert_eq!(result.unresolved, vec!["data_array_tests"]);
    }

    #[test]
    fn multiple_macro_calls_in_one_file_are_all_parsed() {
        let text = "vtk_add_test_cxx(vtkCommonCoreCxx tests1\n  TestA.cxx\n  )\n\
                     vtk_add_test_cxx(vtkCommonCoreCxx tests2\n  TestB.cxx\n  )";
        let result = parse_test_macro_calls(text, "vtk_add_test_cxx", "cxx", CXX_OPTIONS);
        let mut found = names(&result);
        found.sort();
        assert_eq!(found, vec!["TestA", "TestB"]);
    }

    #[test]
    fn comment_lines_are_ignored() {
        let text = "vtk_add_test_cxx(vtkCommonCoreCxx tests\n  # TestIgnored.cxx\n  TestKept.cxx\n  )";
        let result = parse_test_macro_calls(text, "vtk_add_test_cxx", "cxx", CXX_OPTIONS);
        assert_eq!(names(&result), vec!["TestKept"]);
    }

    #[test]
    fn python_macro_uses_python_options_and_py_extension() {
        let text = "vtk_add_test_python(vtkCommonCorePython tests\n  DIRECT_DATA\n  TestFoo.py\n  )";
        let result = parse_test_macro_calls(text, "vtk_add_test_python", "py", PYTHON_OPTIONS);
        assert_eq!(names(&result), vec!["TestFoo"]);
    }

    #[test]
    fn a_call_to_a_different_macro_name_is_ignored() {
        let text = "vtk_add_test_mpi(vtkCommonCoreCxx tests\n  TestMpi.cxx\n  )";
        let result = parse_test_macro_calls(text, "vtk_add_test_cxx", "cxx", CXX_OPTIONS);
        assert!(result.tests.is_empty());
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

```bash
cd rust && cargo test -p xtask cmake_parser:: 2>&1 | tail -20
```

Expected: panic from `todo!()`.

- [ ] **Step 3: Implement `parse_test_macro_calls`**

Replace the `todo!()` body:

```rust
pub fn parse_test_macro_calls(
    text: &str,
    macro_name: &str,
    ext: &str,
    known_options: &[&str],
) -> ParsedCMakeFile {
    let mut result = ParsedCMakeFile::default();
    let stripped = strip_comments(text);
    for call_args in find_macro_call_args(&stripped, macro_name) {
        let tokens = tokenize_args(&call_args);
        // Skip the first two positional args: EXENAME and VARNAME.
        for token in tokens.into_iter().skip(2) {
            classify_token(&token, ext, known_options, &mut result);
        }
    }
    result
}

fn strip_comments(text: &str) -> String {
    text.lines()
        .map(|line| match line.find('#') {
            Some(idx) => &line[..idx],
            None => line,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Finds every `macro_name(...)` call in `text` and returns the raw text between the matching
/// parentheses for each one. Assumes no nested parentheses inside test macro calls (true for
/// every vtk_add_test_cxx/vtk_add_test_python call in the reference tree).
fn find_macro_call_args(text: &str, macro_name: &str) -> Vec<String> {
    let mut calls = Vec::new();
    let mut search_from = 0;
    while let Some(rel_start) = text[search_from..].find(macro_name) {
        let start = search_from + rel_start;
        let after_name = start + macro_name.len();
        // Require the next non-whitespace char to be '(' so we don't match a longer identifier
        // that happens to contain macro_name as a substring.
        let open_paren = match text[after_name..].find(|c: char| !c.is_whitespace()) {
            Some(off) if text[after_name + off..].starts_with('(') => after_name + off,
            _ => {
                search_from = after_name;
                continue;
            }
        };
        let close_paren = match text[open_paren..].find(')') {
            Some(off) => open_paren + off,
            None => break,
        };
        calls.push(text[open_paren + 1..close_paren].to_string());
        search_from = close_paren + 1;
    }
    calls
}

fn tokenize_args(args: &str) -> Vec<String> {
    args.split_whitespace().map(|s| s.to_string()).collect()
}

fn classify_token(token: &str, ext: &str, known_options: &[&str], result: &mut ParsedCMakeFile) {
    if known_options.contains(&token) {
        return;
    }
    if let Some(var_name) = token.strip_prefix("${").and_then(|s| s.strip_suffix('}')) {
        result.unresolved.push(var_name.to_string());
        return;
    }
    let ext_suffix = format!(".{ext}");
    if let Some((left, right)) = token.split_once(',') {
        let name = if left.ends_with(&ext_suffix) {
            left.trim_end_matches(&ext_suffix).to_string()
        } else {
            left.to_string()
        };
        result.tests.push(ParsedTest {
            name,
            raw_token: token.to_string(),
        });
        let _ = right; // kept only for readability; not needed further.
        return;
    }
    if token.ends_with(&ext_suffix) {
        result.tests.push(ParsedTest {
            name: token.trim_end_matches(&ext_suffix).to_string(),
            raw_token: token.to_string(),
        });
    }
    // Anything else (e.g. a stray positional arg) is silently ignored — v1 scope is test
    // extraction, not full argument validation.
}
```

- [ ] **Step 4: Register the module in `main.rs`**

Modify `rust/xtask/src/main.rs` — add `mod cmake_parser;` alongside the existing `mod ledger;`.

- [ ] **Step 5: Run the tests to verify they pass**

```bash
cd rust && cargo test -p xtask cmake_parser::
```

Expected: all 10 tests pass.

- [ ] **Step 6: Commit**

```bash
git add rust/xtask/src/main.rs rust/xtask/src/cmake_parser.rs
git commit -m "xtask: CMake test-macro-call parser for vtk_add_test_cxx/python"
```

---

### Task 3: Reference-tree git integration

**Files:**
- Create: `rust/xtask/src/reference_tree.rs`
- Modify: `rust/xtask/src/main.rs` (add `mod reference_tree;`)

**Interfaces:**
- Consumes: nothing from Tasks 1-2.
- Produces: `pub fn repo_root() -> std::io::Result<std::path::PathBuf>`
- Produces: `pub fn file_exists_in_tree(repo_root: &std::path::Path, rel_path: &str) -> bool`
- Produces: `pub fn blob_sha_at_head(repo_root: &std::path::Path, rel_path: &str) ->
  Option<String>`

- [ ] **Step 1: Write the implementation directly (git shell-out has no pure-logic split worth
  testing in isolation — the test in Step 2 exercises it against this repo's own real git state)**

Write `rust/xtask/src/reference_tree.rs`:

```rust
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn repo_root() -> std::io::Result<PathBuf> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()?;
    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(PathBuf::from(path))
}

pub fn file_exists_in_tree(repo_root: &Path, rel_path: &str) -> bool {
    repo_root.join(rel_path).is_file()
}

pub fn blob_sha_at_head(repo_root: &Path, rel_path: &str) -> Option<String> {
    let output = Command::new("git")
        .arg("rev-parse")
        .arg(format!("HEAD:{rel_path}"))
        .current_dir(repo_root)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let sha = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if sha.is_empty() {
        None
    } else {
        Some(sha)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repo_root_finds_the_real_worktree_root() {
        let root = repo_root().unwrap();
        assert!(root.join("AGENTS.md").is_file());
    }

    #[test]
    fn file_exists_in_tree_is_true_for_a_real_file() {
        let root = repo_root().unwrap();
        assert!(file_exists_in_tree(&root, "rust/Cargo.toml"));
    }

    #[test]
    fn file_exists_in_tree_is_false_for_a_missing_file() {
        let root = repo_root().unwrap();
        assert!(!file_exists_in_tree(&root, "does/not/exist.cxx"));
    }

    #[test]
    fn blob_sha_at_head_returns_a_40_char_sha_for_a_committed_file() {
        let root = repo_root().unwrap();
        let sha = blob_sha_at_head(&root, "rust/Cargo.toml").unwrap();
        assert_eq!(sha.len(), 40);
        assert!(sha.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn blob_sha_at_head_returns_none_for_a_missing_path() {
        let root = repo_root().unwrap();
        assert!(blob_sha_at_head(&root, "does/not/exist.cxx").is_none());
    }
}
```

This is real integration logic against the actual repo, not a fixture — appropriate here because
Task 3's whole job is the git plumbing, and CI always runs inside a real git checkout.

- [ ] **Step 2: Register the module in `main.rs`**

Modify `rust/xtask/src/main.rs` — add `mod reference_tree;` alongside the existing `mod ledger;` and
`mod cmake_parser;`.

- [ ] **Step 3: Run the tests to verify they pass**

```bash
cd rust && cargo test -p xtask reference_tree::
```

Expected: all 5 tests pass. (There is no "make it fail first" step here — Step 1 already contains
the real implementation, since there is no meaningful intermediate `todo!()` state for a function
that's three lines of `std::process::Command` plumbing.)

- [ ] **Step 4: Commit**

```bash
git add rust/xtask/src/main.rs rust/xtask/src/reference_tree.rs
git commit -m "xtask: git-backed reference-tree existence and blob-sha lookups"
```

---

### Task 4: Crate-contains-code detector

**Files:**
- Create: `rust/xtask/src/crates.rs`
- Modify: `rust/xtask/src/main.rs` (add `mod crates;`)

**Interfaces:**
- Consumes: nothing from Tasks 1-3.
- Produces: `pub fn crate_has_code<I: IntoIterator<Item = String>>(file_contents: I) -> bool` (pure)
- Produces: `pub fn crate_has_code_at(src_dir: &std::path::Path) -> std::io::Result<bool>` (I/O
  wrapper used by `main.rs` in Task 6)

- [ ] **Step 1: Write the failing tests**

Write `rust/xtask/src/crates.rs`:

```rust
use std::path::Path;

pub fn crate_has_code<I: IntoIterator<Item = String>>(file_contents: I) -> bool {
    todo!()
}

pub fn crate_has_code_at(src_dir: &Path) -> std::io::Result<bool> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_single_doc_comment_line_is_not_code() {
        let files = vec!["//! Port of VTK::CommonCore. See ROADMAP.md.\n".to_string()];
        assert!(!crate_has_code(files));
    }

    #[test]
    fn blank_lines_are_not_code() {
        let files = vec!["\n\n  \n".to_string()];
        assert!(!crate_has_code(files));
    }

    #[test]
    fn a_real_declaration_is_code() {
        let files = vec!["//! doc\npub struct Foo;\n".to_string()];
        assert!(crate_has_code(files));
    }

    #[test]
    fn code_in_a_second_file_still_counts() {
        let files = vec![
            "//! doc only\n".to_string(),
            "pub fn helper() {}\n".to_string(),
        ];
        assert!(crate_has_code(files));
    }

    #[test]
    fn no_files_is_not_code() {
        let files: Vec<String> = vec![];
        assert!(!crate_has_code(files));
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

```bash
cd rust && cargo test -p xtask crates:: 2>&1 | tail -20
```

Expected: panic from `todo!()`.

- [ ] **Step 3: Implement both functions**

```rust
pub fn crate_has_code<I: IntoIterator<Item = String>>(file_contents: I) -> bool {
    file_contents.into_iter().any(|contents| {
        contents
            .lines()
            .any(|line| !line.trim().is_empty() && !line.trim().starts_with("//"))
    })
}

pub fn crate_has_code_at(src_dir: &Path) -> std::io::Result<bool> {
    let mut file_contents = Vec::new();
    collect_rs_files(src_dir, &mut file_contents)?;
    Ok(crate_has_code(file_contents))
}

fn collect_rs_files(dir: &Path, out: &mut Vec<String>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_rs_files(&path, out)?;
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(std::fs::read_to_string(&path)?);
        }
    }
    Ok(())
}
```

- [ ] **Step 4: Register the module in `main.rs`**

Modify `rust/xtask/src/main.rs` — add `mod crates;` alongside the existing `mod ledger;`,
`mod cmake_parser;`, and `mod reference_tree;`.

- [ ] **Step 5: Run the tests to verify they pass**

```bash
cd rust && cargo test -p xtask crates::
```

Expected: all 5 tests pass.

- [ ] **Step 6: Commit**

```bash
git add rust/xtask/src/main.rs rust/xtask/src/crates.rs
git commit -m "xtask: crate-contains-code detector for the parity assertion"
```

---

### Task 5: The four assertions

**Files:**
- Create: `rust/xtask/src/assertions.rs`
- Modify: `rust/xtask/src/main.rs` (add `mod assertions;`)

**Interfaces:**
- Consumes: `ledger::LedgerRow`, `ledger::module_of` (Task 1); `cmake_parser::ParsedCMakeFile`
  (Task 2).
- Produces: `pub struct Violation { pub assertion: &'static str, pub message: String }`
- Produces: `pub fn check_exists(rows: &[LedgerRow], exists_fn: impl Fn(&str) -> bool) ->
  Vec<Violation>`
- Produces: `pub fn check_fresh(rows: &[LedgerRow], sha_fn: impl Fn(&str) -> Option<String>) ->
  Vec<Violation>`
- Produces: `pub fn check_complete(rows: &[LedgerRow], parsed_files: &[(String, String,
  ParsedCMakeFile)]) -> Vec<Violation>` — each tuple is `(module, file_display_path,
  parsed_result)`.
- Produces: `pub fn check_parity(rows: &[LedgerRow], crates: &[(String, bool)]) -> Vec<Violation>`
  — each tuple is `(crate_dir_name, has_code)`.

- [ ] **Step 1: Write the failing tests — one clean-pass and one positive-control fixture per
  assertion, per issue #41's explicit requirement**

Write `rust/xtask/src/assertions.rs`:

```rust
use crate::cmake_parser::ParsedCMakeFile;
use crate::ledger::{module_of, LedgerRow};

#[derive(Debug, PartialEq)]
pub struct Violation {
    pub assertion: &'static str,
    pub message: String,
}

fn row(original_path: &str, original_test: &str, original_sha: &str, rust_path: &str,
       status: &str, notes: &str) -> LedgerRow {
    LedgerRow {
        original_path: original_path.to_string(),
        original_test: original_test.to_string(),
        original_sha: original_sha.to_string(),
        rust_path: rust_path.to_string(),
        rust_test: String::new(),
        category: "1".to_string(),
        status: status.to_string(),
        notes: notes.to_string(),
    }
}

pub fn check_exists(rows: &[LedgerRow], exists_fn: impl Fn(&str) -> bool) -> Vec<Violation> {
    todo!()
}

pub fn check_fresh(rows: &[LedgerRow], sha_fn: impl Fn(&str) -> Option<String>) -> Vec<Violation> {
    todo!()
}

pub fn check_complete(
    rows: &[LedgerRow],
    parsed_files: &[(String, String, ParsedCMakeFile)],
) -> Vec<Violation> {
    todo!()
}

pub fn check_parity(rows: &[LedgerRow], crates: &[(String, bool)]) -> Vec<Violation> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cmake_parser::ParsedTest;

    // ---- exists ----

    #[test]
    fn exists_passes_when_every_path_is_present() {
        let rows = vec![row("Common/Core/Testing/Cxx/TestFoo.cxx", "TestFoo", "sha", "p", "ported", "")];
        let violations = check_exists(&rows, |_| true);
        assert!(violations.is_empty());
    }

    #[test]
    fn exists_fails_when_a_path_is_missing() {
        let rows = vec![row("Common/Core/Testing/Cxx/TestGone.cxx", "TestGone", "sha", "p", "ported", "")];
        let violations = check_exists(&rows, |_| false);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].assertion, "exists");
    }

    // ---- fresh ----

    #[test]
    fn fresh_passes_when_sha_matches() {
        let rows = vec![row("Common/Core/Testing/Cxx/TestFoo.cxx", "TestFoo", "abc", "p", "ported", "")];
        let violations = check_fresh(&rows, |_| Some("abc".to_string()));
        assert!(violations.is_empty());
    }

    #[test]
    fn fresh_fails_when_sha_has_drifted() {
        let rows = vec![row("Common/Core/Testing/Cxx/TestFoo.cxx", "TestFoo", "abc", "p", "ported", "")];
        let violations = check_fresh(&rows, |_| Some("def".to_string()));
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].assertion, "fresh");
    }

    // ---- complete ----

    #[test]
    fn complete_passes_when_every_parsed_test_has_a_row() {
        let rows = vec![row("Common/Core/Testing/Cxx/TestFoo.cxx", "TestFoo", "sha", "p", "ported", "")];
        let parsed = vec![(
            "Common/Core".to_string(),
            "Common/Core/Testing/Cxx/CMakeLists.txt".to_string(),
            ParsedCMakeFile {
                tests: vec![ParsedTest { name: "TestFoo".to_string(), raw_token: "TestFoo.cxx".to_string() }],
                unresolved: vec![],
            },
        )];
        let violations = check_complete(&rows, &parsed);
        assert!(violations.is_empty());
    }

    #[test]
    fn complete_fails_when_a_registered_test_has_no_row() {
        let rows: Vec<LedgerRow> = vec![row("Common/Core/Testing/Cxx/TestOther.cxx", "TestOther", "sha", "p", "ported", "")];
        let parsed = vec![(
            "Common/Core".to_string(),
            "Common/Core/Testing/Cxx/CMakeLists.txt".to_string(),
            ParsedCMakeFile {
                tests: vec![ParsedTest { name: "TestMissing".to_string(), raw_token: "TestMissing.cxx".to_string() }],
                unresolved: vec![],
            },
        )];
        let violations = check_complete(&rows, &parsed);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].assertion, "complete");
        assert!(violations[0].message.contains("TestMissing"));
    }

    #[test]
    fn complete_fails_on_an_unresolved_variable_with_no_clearing_note() {
        let rows = vec![row("Common/Core/Testing/Cxx/TestFoo.cxx", "TestFoo", "sha", "p", "ported", "")];
        let parsed = vec![(
            "Common/Core".to_string(),
            "Common/Core/Testing/Cxx/CMakeLists.txt".to_string(),
            ParsedCMakeFile {
                tests: vec![ParsedTest { name: "TestFoo".to_string(), raw_token: "TestFoo.cxx".to_string() }],
                unresolved: vec!["data_array_tests".to_string()],
            },
        )];
        let violations = check_complete(&rows, &parsed);
        assert_eq!(violations.len(), 1);
        assert!(violations[0].message.contains("data_array_tests"));
        assert!(violations[0].message.contains("generated:data_array_tests"));
    }

    #[test]
    fn complete_clears_an_unresolved_variable_when_a_row_carries_the_marker() {
        let rows = vec![row(
            "Common/Core/Testing/Cxx/TestDataArrayAPI.cxx",
            "TestDataArrayAPI_vtkCharArray",
            "sha",
            "p",
            "ported",
            "generated:data_array_tests",
        )];
        let parsed = vec![(
            "Common/Core".to_string(),
            "Common/Core/Testing/Cxx/CMakeLists.txt".to_string(),
            ParsedCMakeFile {
                tests: vec![],
                unresolved: vec!["data_array_tests".to_string()],
            },
        )];
        let violations = check_complete(&rows, &parsed);
        assert!(violations.is_empty());
    }

    // ---- parity ----

    #[test]
    fn parity_passes_when_a_ported_row_matches_the_crate() {
        let rows = vec![row(
            "Common/Core/Testing/Cxx/TestFoo.cxx",
            "TestFoo",
            "sha",
            "rust/crates/vtk-common-core/src/foo.rs",
            "ported",
            "",
        )];
        let crates = vec![("vtk-common-core".to_string(), true)];
        let violations = check_parity(&rows, &crates);
        assert!(violations.is_empty());
    }

    #[test]
    fn parity_fails_when_a_crate_has_code_but_no_ported_row() {
        let rows: Vec<LedgerRow> = vec![];
        let crates = vec![("vtk-common-core".to_string(), true)];
        let violations = check_parity(&rows, &crates);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].assertion, "parity");
        assert!(violations[0].message.contains("vtk-common-core"));
    }

    #[test]
    fn parity_ignores_a_crate_with_no_code_yet() {
        let rows: Vec<LedgerRow> = vec![];
        let crates = vec![("vtk-common-core".to_string(), false)];
        let violations = check_parity(&rows, &crates);
        assert!(violations.is_empty());
    }

    #[test]
    fn parity_does_not_count_a_deferred_row_as_ported() {
        let rows = vec![row(
            "Common/Core/Testing/Cxx/TestFoo.cxx",
            "TestFoo",
            "sha",
            "rust/crates/vtk-common-core/src/foo.rs",
            "deferred",
            "phase 2",
        )];
        let crates = vec![("vtk-common-core".to_string(), true)];
        let violations = check_parity(&rows, &crates);
        assert_eq!(violations.len(), 1);
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

```bash
cd rust && cargo test -p xtask assertions:: 2>&1 | tail -20
```

Expected: panics from the four `todo!()` bodies.

- [ ] **Step 3: Implement the four assertion functions**

```rust
pub fn check_exists(rows: &[LedgerRow], exists_fn: impl Fn(&str) -> bool) -> Vec<Violation> {
    rows.iter()
        .filter(|r| !exists_fn(&r.original_path))
        .map(|r| Violation {
            assertion: "exists",
            message: format!(
                "{}: original_path not found in reference tree",
                r.original_path
            ),
        })
        .collect()
}

pub fn check_fresh(rows: &[LedgerRow], sha_fn: impl Fn(&str) -> Option<String>) -> Vec<Violation> {
    rows.iter()
        .filter_map(|r| {
            let current = sha_fn(&r.original_path)?;
            if current != r.original_sha {
                Some(Violation {
                    assertion: "fresh",
                    message: format!(
                        "{} ({}): ledger sha {} but current blob sha {} — re-read and re-port, then update original_sha",
                        r.original_path, r.original_test, r.original_sha, current
                    ),
                })
            } else {
                None
            }
        })
        .collect()
}

pub fn check_complete(
    rows: &[LedgerRow],
    parsed_files: &[(String, String, ParsedCMakeFile)],
) -> Vec<Violation> {
    let mut violations = Vec::new();
    for (module, file_path, parsed) in parsed_files {
        let module_rows: Vec<&LedgerRow> = rows
            .iter()
            .filter(|r| module_of(&r.original_path) == Some(module.as_str()))
            .collect();

        for test in &parsed.tests {
            let has_row = module_rows.iter().any(|r| r.original_test == test.name);
            if !has_row {
                violations.push(Violation {
                    assertion: "complete",
                    message: format!(
                        "{file_path}: test '{}' is registered but has no row in docs/test-mapping.csv",
                        test.name
                    ),
                });
            }
        }

        for var in &parsed.unresolved {
            let marker = format!("generated:{var}");
            let cleared = module_rows.iter().any(|r| r.notes.contains(&marker));
            if !cleared {
                violations.push(Violation {
                    assertion: "complete",
                    message: format!(
                        "{file_path}: unresolved dynamic test list ${{{var}}} — cannot verify completeness. \
                         Manually enumerate its tests into docs/test-mapping.csv, with at least one row's \
                         notes containing '{marker}' once done."
                    ),
                });
            }
        }
    }
    violations
}

pub fn check_parity(rows: &[LedgerRow], crates: &[(String, bool)]) -> Vec<Violation> {
    crates
        .iter()
        .filter(|(_, has_code)| *has_code)
        .filter_map(|(crate_name, _)| {
            let prefix = format!("rust/crates/{crate_name}/");
            let has_ported_row = rows
                .iter()
                .any(|r| r.status == "ported" && r.rust_path.starts_with(&prefix));
            if has_ported_row {
                None
            } else {
                Some(Violation {
                    assertion: "parity",
                    message: format!(
                        "{crate_name}: contains code but has no docs/test-mapping.csv row with status=ported"
                    ),
                })
            }
        })
        .collect()
}
```

- [ ] **Step 4: Register the module in `main.rs`**

Modify `rust/xtask/src/main.rs` — add `mod assertions;` alongside the existing `mod ledger;`,
`mod cmake_parser;`, `mod reference_tree;`, and `mod crates;`.

- [ ] **Step 5: Run the tests to verify they pass**

```bash
cd rust && cargo test -p xtask assertions::
```

Expected: all 13 tests pass.

- [ ] **Step 6: Commit**

```bash
git add rust/xtask/src/main.rs rust/xtask/src/assertions.rs
git commit -m "xtask: implement exists/complete/fresh/parity assertions"
```

---

### Task 6: CLI orchestration

**Files:**
- Modify: `rust/xtask/src/main.rs` (replace the accumulated `mod`-only stub from Tasks 1-5 with the
  full orchestration below)
- Create: `rust/.cargo/config.toml`

**Interfaces:**
- Consumes: every module from Tasks 1-5 (`ledger`, `cmake_parser`, `reference_tree`, `crates`,
  `assertions`).
- Produces: the `xtask` binary's `ledger-check` subcommand — no further Rust interface, this is the
  integration point.

- [ ] **Step 1: Write `rust/.cargo/config.toml`**

```toml
[alias]
xtask = "run --package xtask --"
```

- [ ] **Step 2: Write `rust/xtask/src/main.rs`**

```rust
mod assertions;
mod cmake_parser;
mod crates;
mod ledger;
mod reference_tree;

use std::path::Path;
use std::process::ExitCode;

use assertions::Violation;
use cmake_parser::ParsedCMakeFile;
use ledger::LedgerRow;

const WORKSPACE_CRATES: &[&str] = &[
    "vtk-common-core",
    "vtk-common-math",
    "vtk-common-system",
    "vtk-common-transforms",
    "vtk-common-misc",
    "vtk-common-data-model",
    "vtk-common-execution-model",
];

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("ledger-check") => run_ledger_check(),
        Some(other) => {
            eprintln!("unknown xtask command: {other}");
            eprintln!("usage: cargo xtask ledger-check");
            ExitCode::from(2)
        }
        None => {
            eprintln!("usage: cargo xtask ledger-check");
            ExitCode::from(2)
        }
    }
}

fn run_ledger_check() -> ExitCode {
    let repo_root = match reference_tree::repo_root() {
        Ok(root) => root,
        Err(e) => {
            eprintln!("could not determine repo root via git: {e}");
            return ExitCode::FAILURE;
        }
    };

    let ledger_path = repo_root.join("docs/test-mapping.csv");
    let csv_text = match std::fs::read_to_string(&ledger_path) {
        Ok(text) => text,
        Err(e) => {
            eprintln!("could not read {}: {e}", ledger_path.display());
            return ExitCode::FAILURE;
        }
    };
    let rows = match ledger::parse_ledger(&csv_text) {
        Ok(rows) => rows,
        Err(e) => {
            eprintln!("docs/test-mapping.csv: {e}");
            return ExitCode::FAILURE;
        }
    };

    let mut violations = Vec::new();

    violations.extend(assertions::check_exists(&rows, |p| {
        reference_tree::file_exists_in_tree(&repo_root, p)
    }));
    violations.extend(assertions::check_fresh(&rows, |p| {
        reference_tree::blob_sha_at_head(&repo_root, p)
    }));
    violations.extend(assertions::check_complete(
        &rows,
        &parsed_files_for_started_modules(&repo_root, &rows),
    ));
    violations.extend(assertions::check_parity(&rows, &crate_code_flags(&repo_root)));

    report(&rows, &violations)
}

fn started_modules(rows: &[LedgerRow]) -> Vec<String> {
    let mut modules: Vec<String> = rows
        .iter()
        .filter_map(|r| ledger::module_of(&r.original_path))
        .map(str::to_string)
        .collect();
    modules.sort();
    modules.dedup();
    modules
}

fn parsed_files_for_started_modules(
    repo_root: &Path,
    rows: &[LedgerRow],
) -> Vec<(String, String, ParsedCMakeFile)> {
    let mut out = Vec::new();
    for module in started_modules(rows) {
        for (subdir, macro_name, ext, options) in [
            ("Cxx", "vtk_add_test_cxx", "cxx", cmake_parser::CXX_OPTIONS),
            (
                "Python",
                "vtk_add_test_python",
                "py",
                cmake_parser::PYTHON_OPTIONS,
            ),
        ] {
            let rel_path = format!("{module}/Testing/{subdir}/CMakeLists.txt");
            let full_path = repo_root.join(&rel_path);
            if !full_path.is_file() {
                continue;
            }
            let text = match std::fs::read_to_string(&full_path) {
                Ok(text) => text,
                Err(e) => {
                    eprintln!("warning: could not read {rel_path}: {e}");
                    continue;
                }
            };
            let parsed = cmake_parser::parse_test_macro_calls(&text, macro_name, ext, options);
            out.push((module.clone(), rel_path, parsed));
        }
    }
    out
}

fn crate_code_flags(repo_root: &Path) -> Vec<(String, bool)> {
    WORKSPACE_CRATES
        .iter()
        .map(|name| {
            let src_dir = repo_root.join("rust/crates").join(name).join("src");
            let has_code = crates::crate_has_code_at(&src_dir).unwrap_or(false);
            (name.to_string(), has_code)
        })
        .collect()
}

fn report(rows: &[LedgerRow], violations: &[Violation]) -> ExitCode {
    if violations.is_empty() {
        if rows.is_empty() {
            println!("ledger-check: 0 rows in docs/test-mapping.csv — nothing to check yet");
        } else {
            println!(
                "ledger-check: {} row(s) across {} module(s), no violations",
                rows.len(),
                started_modules(rows).len()
            );
        }
        return ExitCode::SUCCESS;
    }
    for v in violations {
        println!("[{}] {}", v.assertion, v.message);
    }
    println!("ledger-check: {} violation(s)", violations.len());
    ExitCode::FAILURE
}
```

- [ ] **Step 3: Build and run against the real repo**

```bash
cd rust && cargo build -p xtask && cargo xtask ledger-check
```

Expected: exit 0, printing `ledger-check: 0 rows in docs/test-mapping.csv — nothing to check yet`
(the ledger is empty today — no module has started porting).

- [ ] **Step 4: Run the full test suite for the crate**

```bash
cd rust && cargo test -p xtask
```

Expected: all tests from Tasks 1-5 still pass (this task added no new unit tests of its own —
Task 7's controller-executed step is what verifies this task's own logic end-to-end).

- [ ] **Step 5: Commit**

```bash
git add rust/.cargo/config.toml rust/xtask/src/main.rs
git commit -m "xtask: wire ledger-check CLI, alias cargo xtask"
```

---

### Task 7: CI wiring, docs, and controller-executed positive control

**Files:**
- Modify: `.github/workflows/rust-checks.yml` (add a `xtask-ledger-check` job)
- Modify: `AGENTS.md:180-184` § Required checks (flip the `cargo xtask ledger-check` bullet from
  "Not yet wired" to "Live today", matching the `cargo-check-wasm32` bullet's phrasing just above
  it)
- Modify: `AGENTS.md` § Commands (move `ledger-check` out of the "don't exist yet" list)
- Modify: `AGENTS.md` § The test-mapping ledger, § What CI checks about the ledger (document the
  `generated:<var>` clearing convention)

**Interfaces:**
- Consumes: the `cargo xtask ledger-check` command from Task 6.
- Produces: nothing further — this is the final wiring and verification task.

- [ ] **Step 1: Add the CI job**

Modify `.github/workflows/rust-checks.yml` — append a new job after `cargo-check-wasm32`:

```yaml
  xtask-ledger-check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - uses: dtolnay/rust-toolchain@stable

      - uses: Swatinem/rust-cache@v2
        with:
          workspaces: rust

      - name: cargo xtask ledger-check
        working-directory: rust
        run: cargo xtask ledger-check
```

No special `fetch-depth` is needed: `blob_sha_at_head` only reads `HEAD`'s tree, which a shallow
`actions/checkout@v4` (the default) already includes in full.

- [ ] **Step 2: Flip the § Required checks bullet to "Live today"**

In `AGENTS.md` § Required checks, find:

```
- `cargo xtask ledger-check` — the four ledger assertions (*exists*, *complete*, *fresh*,
  *parity*); see **The test-mapping ledger**. Cheap, and it fails loudly the moment the ledger
  stops describing the reference tree instead of letting it drift. Not yet wired — dependency-order
  Step 3, see `docs/superpowers/specs/2026-08-06-autonomous-operation-design.md` § Dependency
  order.
```

Replace the last sentence so the bullet reads:

```
- `cargo xtask ledger-check` — the four ledger assertions (*exists*, *complete*, *fresh*,
  *parity*); see **The test-mapping ledger**. Cheap, and it fails loudly the moment the ledger
  stops describing the reference tree instead of letting it drift. Live today
  (`xtask-ledger-check` in `.github/workflows/rust-checks.yml`), but not yet marked required in
  branch protection — see § Required checks' opening paragraph. No issue is filed for the
  required-status-check wiring itself yet (confirmed via `gh issue list` while writing this plan:
  no open issue covers it); file one before starting that work rather than inventing a number here.
```

- [ ] **Step 3: Update `AGENTS.md` § Commands**

Find the paragraph that currently reads (approximately):

```
`cargo xtask` commands (`ledger-check`, `test-mapping-report`, `upstream-diff`) don't exist yet —
they are dependency-order Step 3, see `docs/superpowers/specs/2026-08-06-autonomous-operation-design.md`
§ Dependency order.
```

Replace it with:

```
cargo xtask ledger-check
```

`cargo xtask test-mapping-report` and `cargo xtask upstream-diff` don't exist yet — the latter is
tracked as [#44](https://github.com/rotnov/vtk.rs/issues/44); the former has no filed issue.

(Place the `cargo xtask ledger-check` line inside the existing fenced command block alongside
`cargo build --workspace` etc.; the explanatory paragraph follows the code block, matching the
existing style in that section.)

- [ ] **Step 4: Document the `generated:<var>` clearing convention**

In `AGENTS.md` § The test-mapping ledger, immediately after the paragraph explaining the `notes`
column's `status` values, add:

```
When a module's `Testing/*/CMakeLists.txt` splices a CMake variable into a test-macro call instead
of listing literal test names (e.g. `${data_array_tests}`, built via a `foreach` loop and
`configure_file()` templating), `ledger-check`'s **complete** assertion cannot resolve it — this is
permanently out of scope for the checker, not a bug. Enumerate that variable's tests by hand (read
the generating CMake code, e.g. `add_data_array_test`, to find them) and add one ledger row per
test as usual. Once done, at least one of those rows' `notes` must contain `generated:<variable
name>` (e.g. `generated:data_array_tests`) — this is what tells `ledger-check` the variable has
been manually accounted for. Without that marker, `complete` reports the variable as unresolved on
every run.
```

In `AGENTS.md` § What CI checks about the ledger, in the **complete** bullet, after the existing
text, add one sentence: "A `${variable}` spliced into a test-macro call instead of a literal test
name is flagged as unresolved rather than silently skipped — see § The test-mapping ledger for how
to clear it."

In `AGENTS.md` § The test-mapping ledger, in the sentence(s) defining the `original_test` column,
replace the "registered CTest name" wording with a precise pin so a future porter doesn't write the
executable-prefixed CTest name (e.g. `vtkCommonCoreCxx-TestArrayAPI`) and get spurious `complete`
failures. Add, adjacent to the `original_test` definition:

```
`original_test` is the bare test name as it appears in the `Testing/*/CMakeLists.txt` macro call
(e.g. `TestArrayAPI`), **not** the executable-prefixed name CTest actually registers (e.g.
`vtkCommonCoreCxx-TestArrayAPI`). `cargo xtask ledger-check`'s **complete** assertion parses macro
calls directly and matches against this bare form.
```

- [ ] **Step 5: Controller-executed verification — confirm the day-one green pass**

```bash
cd rust && cargo xtask ledger-check
```

Expected: exit 0, `ledger-check: 0 rows in docs/test-mapping.csv — nothing to check yet`.

- [ ] **Step 6: Controller-executed positive control — confirm each assertion can actually fail
  against the real repo, per issue #41's explicit trap-avoidance requirement**

This step edits `docs/test-mapping.csv` in the working tree only, verifies the failure, then
reverts — nothing here is committed.

```bash
# Append one row naming a real, existing reference-tree file (so `exists` passes) with a
# deliberately wrong sha (so `fresh` fires). This also makes Common/Core a started module, which
# simultaneously exercises `complete` against every other test in that file's CMakeLists.txt,
# including the unresolved `${data_array_tests}` variable (no clearing-marker row present yet).
printf 'Common/Core/Testing/Cxx/TestArrayAPI.cxx,TestArrayAPI,0000000000000000000000000000000000000000,rust/crates/vtk-common-core/src/fake.rs,fake_test,1,ported,\n' >> docs/test-mapping.csv
cd rust && cargo xtask ledger-check; cd ..
```

Expected: nonzero exit, with all of: a `[fresh]` violation for `TestArrayAPI.cxx` (the sha doesn't
match HEAD's real blob sha), one `[complete]` violation per test registered in
`Common/Core/Testing/Cxx/CMakeLists.txt` that has no ledger row of its own, and a `[complete]`
violation for the unresolved `${data_array_tests}` variable naming the missing
`generated:data_array_tests` marker. This is the first real-volume look at whether the unresolved-
token message reads usefully outside a unit fixture — if it doesn't, fix the message wording before
moving on, since Task 5's unit tests only exercised it in isolation.

```bash
# Revert the working-tree edit — nothing from this step is committed.
git checkout -- docs/test-mapping.csv
```

Then confirm parity's positive control by temporarily adding a real line of code to a crate that
currently has none:

```bash
echo 'pub struct TemporaryProbe;' >> rust/crates/vtk-common-core/src/lib.rs
cd rust && cargo xtask ledger-check; cd ..
```

Expected: nonzero exit, a `[parity]` violation naming `vtk-common-core` (it now contains code but
has no `status=ported` row).

```bash
git checkout -- rust/crates/vtk-common-core/src/lib.rs
```

- [ ] **Step 7: Run the full workspace test suite**

```bash
cd rust && cargo test --workspace --all-features && cargo clippy --workspace --all-targets --all-features -- -D warnings && cargo fmt --all --check
```

Expected: all green.

- [ ] **Step 8: Commit**

```bash
git add .github/workflows/rust-checks.yml AGENTS.md
git commit -m "ci: wire cargo xtask ledger-check as a required-check candidate"
```

- [ ] **Step 9: Open a PR from this issue branch, per `AGENTS.md` § Change workflow, closing
  issue #41**
