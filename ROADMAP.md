# Roadmap

Status legend: `[ ]` not started · `[~]` in progress · `[x]` done (all category-1/2 tests for
the module pass; see `AGENTS.md` for test categories)

## Snapshot (reference: VTK v9.6.2)

- 278 modules total (`vtk.module` files), organized under `Common`, `Filters`, `IO`,
  `Rendering`, `Interaction`, `Imaging`, `Charts`, `Views`, `Domains`, `Parallel`, `Geovis`,
  `Infovis`, `Accelerators`, `Web`, `Wrapping`, `GUISupport`.
- 270 `Testing/` directories, 2388 C++ test files, 921 Python test files.
- 73 `Testing/Cxx` dirs register tests with `NO_DATA NO_VALID NO_OUTPUT` (pure-logic, no
  external baseline) — these are the cheapest to port and drive the phase-1 TDD loop.
- 173 `DATA{...}` (ExternalData baseline) references repo-wide — mostly `Rendering*`, some
  `Filters*`/`IO*`. Deferred per module until the relevant phase.
- Module order below follows each module's `DEPENDS` in its `vtk.module` file (verified by
  reading the actual files, not assumed) — this is a real topological slice of VTK's own build
  graph, not a guess.

Most of VTK's 278 modules are long-tail format/domain support (CGNS, ADIOS2, USD, OpenVR,
MySQL/ODBC/PostgreSQL readers, ...). This roadmap only sequences the core that everything else
sits on top of. Domain modules get prioritized ad hoc, after Phase 4, based on what's actually
needed.

## Phase 0 — Bootstrap

- [ ] `rust/` Cargo workspace skeleton, empty crates for Phase 1 modules, CI in
      `.github/workflows/` running the required checks from `AGENTS.md` § Change workflow:
      `cargo test`, `cargo clippy -D warnings`, `cargo fmt --check`, and the coverage gate
      (`cargo llvm-cov --fail-under-lines 100 --fail-under-functions 100`, see
      `docs/decisions/0001-test-coverage-metric.md`).
- [ ] Protect `master` on `rotnov/vtk.rs`: PR required, required status checks, no direct or
      force pushes. **Blocked on the owner** — this is a GitHub repo setting needing their
      token, not something an agent can commit.
- [ ] `docs/test-mapping.csv` schema + a small script/xtask to summarize coverage
      (ported/passing/deferred counts per module).
- [ ] Decide and record (in `docs/decisions/`) the numeric-array storage strategy for
      `vtk-common-core` (this determines a lot downstream): enum-of-typed-`Vec` vs generic
      struct, and how `vtkDataArray`'s runtime type dispatch (`vtkTemplateMacro`) maps to Rust.

## Phase 1 — `Common*` (no VTK-internal deps beyond this group)

Dependency-ordered:

1. `vtk-common-core` (`VTK::CommonCore`) — depends only on third-party (`fast_float`, `fmt`,
   `vtksys`, ...), no other VTK module. True root. `vtkMath`, `vtkPoints`, `vtkDataArray` family,
   object/array base types.
2. `vtk-common-math`, `vtk-common-transforms`, `vtk-common-system`, `vtk-common-misc` — depend
   on `CommonCore` only, largely independent of each other. Can be done in parallel.
3. `vtk-common-data-model` (`VTK::CommonDataModel`) — depends on `CommonCore`, `CommonMath`,
   `CommonTransforms` (+ private: `CommonMisc`, `CommonSystem`). Cells, `vtkPolyData`,
   `vtkUnstructuredGrid`, geometry primitives. The real foundation everything else builds on.
4. `vtk-common-execution-model` (`VTK::CommonExecutionModel`) — the `vtkAlgorithm` pipeline
   (`Update()`, dirty-flag propagation). Needed before any filter exists.

**Exit criteria**: all category-1 (`NO_DATA NO_VALID NO_OUTPUT`) tests from
`Common/Core/Testing/Cxx` and `Common/DataModel/Testing/Cxx` ported and passing.

## Phase 2 — IO (Legacy + XML), round-trip tests as acceptance criteria

1. `vtk-io-legacy` (`VTK::IOLegacy`) — `.vtk` reader/writer.
2. `vtk-io-xml` (`VTK::IOXML`) — `.vtp`/`.vtu`/etc. reader/writer.

IO is deliberately phase 2, ahead of filters: its tests are self-verifying round-trips
(write, read back, compare — see `IO/Legacy/Testing/Cxx`, e.g.
`TestLegacyCompositeDataReaderWriter.cxx`) with no rendering or external baseline dependency,
so they're a strong, cheap regression net, and file compatibility with real-world `.vtk`/`.vtu`
files is usually the actual practical goal of a port like this.

**Exit criteria**: round-trip tests pass against files produced by *real* upstream VTK (not
just self-produced files) — use the reference tree's build or a system VTK install to generate
fixtures once, check the fixtures into `rust/crates/vtk-io-*/tests/fixtures/`.

## Phase 3 — `Filters` (Core, General, Sources, Geometry)

1. `vtk-filters-core` (`VTK::FiltersCore`) — depends on `CommonCore/DataModel/ExecutionModel/Misc`.
2. `vtk-filters-general`, `vtk-filters-sources`, `vtk-filters-geometry`.

These are the first algorithms (clip, decimate, contour, geometry extraction). Pipeline
execution model from Phase 1 gets exercised for real here.

**Exit criteria**: category-1 tests pass; category-2 tests that only need `Common*`+`IO*`
fixtures (no image baseline) pass.

## Phase 4 — Rendering (core)

- [ ] Pick and record the backend decision in `docs/decisions/` — default assumption: `wgpu`,
      not a port of `RenderingOpenGL2`. `vtk-rendering-core` defines the same conceptual
      surface (`Renderer`, `RenderWindow`, `Actor`, `Mapper`) but the backend is new code, not a
      translation of `Rendering/OpenGL2`.
- [ ] `vtk-rendering-core` — scene graph, camera, actor/mapper/property model.
- [ ] First category-3 tests become portable here: prefer converting pixel-comparison tests to
      data/geometry assertions (compare the mesh fed to the renderer, not the rendered pixels)
      per `AGENTS.md`; keep true pixel-diff tests as a small, explicitly-tagged minority.

## Phase 5 — Interaction / Widgets

`vtk-interaction-style` — camera/trackball interactors. Depends on Phase 4.

## Phase 6+ — Long tail

Domain-specific IO and filters (CGNS, Exodus, ADIOS2, USD, OpenVR/OpenXR, SQL backends, etc.) —
278 modules total, most of this bucket. Prioritize based on actual need, not completeness; this
roadmap intentionally stops sequencing here.

## Open questions

- **Rendering backend**: `wgpu` assumed above; not yet formally decided. Record the decision
  once made.
- **FFI reference-testing**: worth keeping a `cxx`-based bridge to a real VTK build (system
  package or the reference tree, built) purely as a test oracle — run the same fixture through
  upstream VTK and the Rust port, diff outputs — rather than trusting hand-transcribed expected
  values for anything non-trivial. Not required for Phase 1, worth setting up before Phase 3.
- **License**: VTK is BSD-3-Clause-Sandia-USGov (see root `Copyright.txt`). The Rust port
  should carry compatible licensing; confirm the exact SPDX identifier to use for new files
  under `rust/` before the first real release (not a blocker for development).
- **Upstream sync cadence**: pinned at `v9.6.2`. No policy yet for when/how to re-pin — revisit
  once Phase 1–2 are stable.

## On "port the tests first" as a methodology (see `AGENTS.md` § Testing strategy)

Works well, with caveats, confirmed against the actual test suite rather than assumed:

- It's a strong fit for the 73-ish `NO_DATA NO_VALID NO_OUTPUT` test directories — genuine
  unit tests of data structures and math, no framework mismatch (VTK's
  `int Test(int, char*[])` -> error count maps 1:1 onto `#[test]` + `assert!`). This is real
  red/green TDD, not aspirational.
- For IO and filter tests it's better described as **characterization/acceptance testing**
  than strict TDD: these are integration-shaped (build a small pipeline, run it, check the
  result), and porting them *before* the module exists mostly means writing them
  `#[ignore]`d as a spec, then implementing to satisfy them — still valuable, just not
  tiny-increment TDD.
- Image-comparison tests (173 `DATA{...}` sites) are the real exception: don't port them as-is
  before a renderer exists, and even then prefer converting pixel-diffs to data/geometry
  assertions where the test's actual intent allows it.
- The one addition beyond "just port the tests": keep `docs/test-mapping.csv` as a traceability
  ledger (original path -> rust path -> category -> status). Without it, "how much of VTK is
  actually ported" is unanswerable once this passes a few hundred tests, and upstream parity
  claims aren't auditable.
