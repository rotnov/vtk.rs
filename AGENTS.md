# AGENTS.md

## Project

**vtk.rs** — a port of [VTK](https://vtk.org) (Visualization Toolkit) from C++ to Rust.

This repository is a fork of upstream VTK (`Kitware/VTK`, pinned at `v9.6.2`). The original
C++ source tree is kept at the repository root, unmodified, as a permanent reference. The Rust
port is built incrementally in `rust/`, module by module, alongside it.

Upstream: https://github.com/Kitware/VTK
Fork: https://github.com/rotnov/vtk.rs

## This project is AI-autopilot

**There is no human writing code in this repository.** All implementation, porting decisions,
tests, commits, and this documentation are produced and maintained by AI agents (Claude)
operating autonomously. The human owner (rotnov) sets direction and priorities but does not
hand-write or line-review code in the traditional sense.

Practical implications for any agent working here:

- Don't wait for human code review before proceeding. The correctness gate is the ported test
  suite (see **Testing strategy** below), not human sign-off on a diff.
- Own the decision, don't defer it. If a design choice isn't specified in this file or
  `ROADMAP.md`, make the call, document it (in a doc-comment or an ADR-style note under
  `docs/decisions/` if it's significant), and move on.
- Keep the project self-explanatory for the *next* agent, not for a human maintainer skimming
  a PR. Future context comes from another agent reading this repo cold, not from institutional
  memory. Write commit messages, comments, and `ROADMAP.md` status updates accordingly.
- Update `ROADMAP.md` as you complete work. It is the source of truth for what's done, not this
  file.
- If genuinely blocked on a decision only a human can make (licensing, scope, credentials),
  stop and surface it clearly rather than guessing silently.

## Repository layout

```
/                     original VTK C++ source (Common/, Filters/, IO/, Rendering/, Testing/,
                       CMake/, ThirdParty/, ...) — READ-ONLY reference, pinned at v9.6.2.
                       Do not modify. Used to (a) read original implementations while porting,
                       (b) source tests to port, (c) diff behavior against.
rust/                 the actual Rust port (Cargo workspace). All new work happens here.
  Cargo.toml           workspace manifest
  crates/
    vtk-common-core/
    vtk-common-math/
    vtk-common-transforms/
    vtk-common-misc/
    vtk-common-system/
    vtk-common-data-model/
    vtk-common-execution-model/
    vtk-io-legacy/
    vtk-io-xml/
    vtk-filters-core/
    ...                one crate per ported VTK module, see ROADMAP.md for order
docs/
  test-mapping.csv      traceability: original VTK test path -> rust test -> status
  decisions/             short ADR-style notes for non-obvious design calls
ROADMAP.md              module porting order, phase status, open questions
AGENTS.md               this file
```

Never edit files outside `rust/` and `docs/` — the root tree must stay byte-identical to
upstream `v9.6.2` so it remains a trustworthy reference and diffs against future VTK releases
stay meaningful.

## Upstream version

Pinned at `v9.6.2` (latest stable tag as of 2026-08; `v9.7.0` was still in `rc` at pinning
time). Do not silently move to a newer tag — bumping the pinned version is a deliberate
decision that invalidates parts of the test-mapping and should be recorded in
`docs/decisions/`.

## Rust workspace conventions

- **Crate naming**: `vtk-<kebab-case-module-path>`, mirroring the VTK module's `NAME` in its
  `vtk.module` file. `VTK::CommonCore` -> `vtk-common-core`, `VTK::CommonDataModel` ->
  `vtk-common-data-model`, `VTK::FiltersCore` -> `vtk-filters-core`, etc.
- **Dependency graph**: a crate's `Cargo.toml` dependencies must mirror that module's `DEPENDS`
  in its `vtk.module` file (found in the reference tree, e.g. `Common/DataModel/vtk.module`).
  Don't introduce a dependency VTK's own module graph doesn't have — it's a strong signal
  something is being ported at the wrong layer.
- **No 1:1 class-for-class translation.** VTK's OOP model (intrusive ref-counting via
  `vtkObjectBase`, virtual dispatch, RTTI via `vtkTypeMacro`) doesn't map onto Rust directly.
  Prefer: `enum` + traits for polymorphism where VTK uses a small closed set of subclasses
  (e.g. `vtkDataArray` subclasses), generics where VTK uses `vtkTemplateMacro`-style dispatch,
  plain structs + `Result` instead of ref-counted mutable base objects where the original
  inheritance was only there for shared bookkeeping.
- **Errors**: use `Result<T, E>` with a per-crate error enum. VTK's `vtkErrorMacro` /
  `vtkOutputWindow` observer pattern is a C++-ism, not a contract to preserve.
- **`unsafe`**: allowed only for verified perf-critical hot paths (e.g. bulk array access),
  must be justified with a `// SAFETY:` comment, and must have a safe wrapper at the crate's
  public boundary.
- **No rendering dependency creep**: `Common*`, `Filters*`, and `IO*` crates must not depend on
  a rendering backend. Rendering (`Rendering*` phase, see `ROADMAP.md`) is additive on top.

## Testing strategy: port VTK's tests first, then implement

This is the core workflow for every module. It's a test-driven approach, but adapted to the
reality of what VTK's test suite actually looks like (see the breakdown below) — not literal
red/green/refactor on every class.

VTK's tests are plain functions, `int TestName(int, char*[])`, run by CTest, returning an error
count (`0` = pass). That maps directly onto Rust `#[test]` + `assert!`/`assert_eq!` with no
framework impedance mismatch — this is what makes porting tests first tractable.

Reference-tree stats (v9.6.2, for calibration): 278 modules, 270 `Testing/` directories, 2388
C++ test files, 921 Python test files. Not all of these are the same *kind* of test — triage
before porting:

1. **Pure-logic tests** — registered with `NO_DATA NO_VALID NO_OUTPUT` in the module's
   `Testing/Cxx/CMakeLists.txt` (73 of 270 Testing dirs use this combination). No external data,
   no baseline image, no rendering. Example: `Common/DataModel/Testing/Cxx/TestColor.cxx`.
   **Port these first, verbatim, one `#[test]` per original test function.** Keep a comment
   pointing back to the source file. This is the real "write the test, then make it pass" loop.

2. **Data / round-trip tests** — e.g. everything under `IO/Legacy/Testing/Cxx` and
   `IO/XML/Testing/Cxx`: write a file, read it back, assert equality. Self-verifying, no
   external baseline needed beyond what the test generates itself. Port these once the
   relevant data model + I/O module exists; they're the acceptance criteria for that module,
   not a unit-by-unit TDD loop.

3. **External-data / image-comparison tests** — reference an `ExternalData` baseline via
   `DATA{...}` in `CMakeLists.txt` (173 occurrences repo-wide, concentrated in `Rendering*` and
   some `Filters*`/`IO*` modules). These need either a fetched baseline image+pixel-diff
   (meaningless before a renderer exists) or a numeric/data baseline. Don't port these until
   the corresponding phase (rendering, or the specific filter) is reached. Where the test's
   *intent* is really "does this filter produce the right mesh" rather than "does this pixel
   match", prefer porting it as a data assertion against the mesh/array output instead of a
   pixel comparison — cheaper, deterministic, and catches the same regressions.

Workflow per module:

1. Read the module's `vtk.module` (deps) and `Testing/Cxx/CMakeLists.txt` (test list + flags)
   in the reference tree.
2. Classify every test into one of the three buckets above; record it in
   `docs/test-mapping.csv` (`original_path,rust_path,category,status`).
3. Port category-1 tests first as failing `#[test]`s (red).
4. Implement the minimum to make them pass (green), refactor.
5. Port category-2 tests for the same module; they should already pass once the module is
   complete, or expose gaps category-1 tests missed.
6. Category-3 tests stay `status=deferred` in the mapping file until their phase.

Don't try to port all 2388+921 tests up front — triage and port per-module, in the order
`ROADMAP.md` defines.

## Commands

Not yet bootstrapped — first agent to touch `rust/` should set up the Cargo workspace and
replace this section with real `cargo build` / `cargo test` / `cargo xtask test-mapping-report`
commands.

## Do / Don't

- Do check the reference tree and its tests before implementing anything — don't guess VTK
  behavior from general knowledge of the library.
- Do keep commits scoped to one module/feature.
- Do update `docs/test-mapping.csv` in the same commit as any test you port.
- Don't modify anything outside `rust/` and `docs/`.
- Don't add a rendering dependency to non-rendering crates.
- Don't port `Remote`/`ThirdParty`-gated or niche domain modules (CGNS, ADIOS2, USD, OpenVR,
  ...) before the core (`Common*`/`Filters Core+General`/`IO Legacy+XML`/`Rendering Core`) is
  solid — see `ROADMAP.md` phase order.
