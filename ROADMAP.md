# Roadmap

Status legend: `[ ]` not started · `[~]` in progress · `[x]` done (all category-1/2 tests for
the module pass; see `AGENTS.md` for test categories)

## Snapshot (reference: VTK v9.6.2)

- 278 modules total (`vtk.module` files), organized under `Common`, `Filters`, `IO`,
  `Rendering`, `Interaction`, `Imaging`, `Charts`, `Views`, `Domains`, `Parallel`, `Geovis`,
  `Infovis`, `Accelerators`, `Web`, `Wrapping`, `GUISupport`.
- 229 directories named `Testing/`; 2388 C++ test files under `Testing/Cxx/`; 921 Python test
  files under `Testing/Python/`.
- 173 `DATA{...}` (ExternalData baseline) references repo-wide — mostly `Rendering*`, some
  `Filters*`/`IO*`. Deferred per module until the relevant phase.
- Module order below follows each module's `DEPENDS` in its `vtk.module` file, read per module
  and quoted inline in each phase so it can be re-checked without redoing the work.

Counts verified against the `v9.6.2` tag now actually vendored here. They are sensitive to how
you count, so the commands are part of the claim:

```sh
find . -name vtk.module -not -path './.git/*' | wc -l                    # 278
find . -type d -name Testing -not -path './.git/*' | wc -l               # 229
find . -path '*/Testing/Cxx/*' -name '*.cxx' -not -path './.git/*' | wc -l    # 2388
find . -path '*/Testing/Python/*' -name '*.py' -not -path './.git/*' | wc -l  # 921
grep -ro 'DATA{' --include=CMakeLists.txt . | wc -l                      # 173
```

Re-run them after any change to the pinned upstream version; the figures are not portable across
versions.

**Sizing caveat: pure-logic tests cannot be counted by grep.** `vtk_add_test_cxx` accepts its
flags in two forms, and an earlier version of this file measured only one:

- *block-level*, applying to a whole list — `vtk_add_test_cxx(tgt tests` / `NO_DATA NO_VALID
  NO_OUTPUT` / `<many tests>)`;
- *per-test*, comma-separated — `TestFoo.cxx,NO_DATA,NO_VALID`.

Grepping the first form counts **lines, not directories, and not tests** — in `v9.6.2` it hits 70
times across 67 directories, which an earlier version of this file reported as "73 of 270
directories". Worse, one hit covers a whole block: `Common/Core/Testing/Cxx/CMakeLists.txt`
registers ~95 tests under a single such block, `Common/DataModel/Testing/Cxx` roughly 133. The
per-test comma form is invisible to that grep entirely despite being widespread. So the figure
understated Phase 1 by about an order of magnitude. Classify by reading each `CMakeLists.txt` and
handling both forms — see `AGENTS.md` § Testing strategy.

Most of VTK's 278 modules are long-tail format/domain support (CGNS, ADIOS2, USD, OpenVR,
MySQL/ODBC/PostgreSQL readers, ...). This roadmap only sequences the core that everything else
sits on top of. Domain modules get prioritized ad hoc, after Phase 4, based on what's actually
needed.

## Phase 0 — Bootstrap

- [x] `rust/` Cargo workspace skeleton, empty crates for Phase 1 modules, CI in
      `.github/workflows/` running `cargo test`, `cargo clippy -D warnings`, and
      `cargo fmt --check` from `AGENTS.md` § Change workflow — done: 7 crates matching Phase 1's
      `DEPENDS` below, three jobs live in `.github/workflows/rust-checks.yml`, each proven by a
      positive-control smoke test to fail independently — see
      `docs/superpowers/plans/2026-08-07-rust-workspace-ci.md`.
- [ ] Coverage gate (`cargo llvm-cov --fail-under-lines 100 --fail-under-functions 100`) wired
      into CI. Not part of the workspace-skeleton bullet above: an entire workspace with zero
      executing tests makes the tool hard-error rather than report 100%, verified empirically —
      see `docs/decisions/0001-test-coverage-metric.md`'s 2026-08-07 amendment and
      `docs/lessons/0010-adr-tool-claim-never-run.md`. Wire this job in the same PR as Phase 1's
      first crate with an actually-executing test, not before. Tracked as issue #42.
- [x] `cargo check --target wasm32-unknown-unknown` wired into CI for `Common*`/`Filters*`
      crates, per `AGENTS.md` § WebAssembly — done: `cargo-check-wasm32` job in
      `.github/workflows/rust-checks.yml`, covering all 7 `vtk-common-*` crates via explicit `-p`
      flags (`Filters*` don't exist yet — add them to the job when they do), proven by a
      positive-control smoke test to fail on a real wasm-incompatible API — see
      `docs/superpowers/plans/2026-08-07-wasm-check-common.md`.
- [x] Protect `master` on `rotnov/vtk.rs` — done: PR required, 0 approvals, no direct or force
      pushes, no deletion, linear history, `enforce_admins` on.
- [ ] Add the required status checks to that protection once CI exists. Until then "green CI is
      the review" is an honour system, since there is nothing to require. Tracked as issue #21;
      owner has pre-authorized this repo-settings change to run as soon as `cargo xtask
      ledger-check` (below) lands and is green.
- [x] `docs/test-mapping.csv` schema — done, see `AGENTS.md` § The test-mapping ledger.
- [ ] `cargo xtask ledger-check` — the four assertions *exists* / *complete* / *fresh* / *parity*
      — wired into CI as a required check. See `docs/decisions/0003-upstream-sync-strategy.md` and
      `docs/superpowers/specs/2026-08-06-autonomous-operation-design.md` § 4 "The parity gate
      becomes real". Tracked as issue #41.
- [ ] `cargo xtask upstream-diff <old-tag> <new-tag>` — bucket the upstream diff by module into
      tests added / removed / changed and sources changed in ported modules. Not needed until the
      first version bump, but it is what makes that bump reviewable rather than opaque. Tracked as
      issue #44.
- [ ] Decide and record (in `docs/decisions/`) the numeric-array storage strategy for
      `vtk-common-core` (this determines a lot downstream): enum-of-typed-`Vec` vs generic
      struct, and how `vtkDataArray`'s runtime type dispatch (`vtkTemplateMacro`) maps to Rust.
      Tracked as issue #43.
- [x] GitHub Pages status dashboard — one live stat, percent of catalogued tests with
      `status=ported` in `docs/test-mapping.csv`, rebuilt on every push to `master` via
      `.github/workflows/pages.yml` (generator: `.github/scripts/generate_dashboard.py`). See
      `docs/superpowers/plans/2026-08-07-status-dashboard.md`. Live at
      https://rotnov.github.io/vtk.rs/, confirmed showing "0 catalogued tests yet" after the
      merge of #39 (commit bcdb7231b1520c24ef3c70a541e65b87592e07c1).

## Phase 1 — `Common*` (no VTK-internal deps beyond this group)

Dependency-ordered. Each entry lists the module's `DEPENDS` verbatim from its `vtk.module`;
third-party deps are named because they are work items too, not free.

- [ ] 1. `vtk-common-core` (`VTK::CommonCore`) — `DEPENDS: fast_float, fmt, kwiml, nlohmannjson,
      scn, token, vtksys` (+ optional `loguru`). No VTK-internal deps: the true root. `vtkMath`,
      `vtkPoints`, `vtkDataArray` family, object/array base types. Tracked as issue #45; blocked
      on the numeric-array storage ADR, issue #43.
- [ ] 2. `vtk-common-math` (`VTK::CommonMath`) — `DEPENDS: CommonCore, kissfft`. kissfft is
      replaced by `rustfft` + `realfft`, see `docs/decisions/0002-fft-backend.md`. Note the work
      is in `vtkFFT`'s signal-processing layer (`Spectrogram`, `Csd`, window generators, scaling
      and octave-band helpers), not in wiring up the transform. Tracked as issue #46.
- [ ] 2. `vtk-common-system` (`VTK::CommonSystem`) — `DEPENDS: CommonCore`. Independent of
      `CommonMath`; can run in parallel with it. Tracked as issue #47.
- [ ] 3. `vtk-common-transforms` (`VTK::CommonTransforms`) — `DEPENDS: CommonCore, CommonMath`.
      Tracked as issue #48.
- [ ] 3. `vtk-common-misc` (`VTK::CommonMisc`) — `DEPENDS: CommonCore, CommonMath` (+ private
      `exprtk`, backing `vtkFunctionParser` — a full expression parser, size it before starting).
      Tracked as issue #49.
- [ ] 4. `vtk-common-data-model` (`VTK::CommonDataModel`) — `DEPENDS: CommonCore, CommonMath,
      CommonTransforms` (+ private `CommonMisc`, `CommonSystem`, `pegtl`, `pugixml`). Cells,
      `vtkPolyData`, `vtkUnstructuredGrid`, geometry primitives. The real foundation everything
      else builds on. Tracked as issue #50.
- [ ] 5. `vtk-common-execution-model` (`VTK::CommonExecutionModel`) — `DEPENDS: CommonCore,
      CommonDataModel` (+ private `CommonMisc`, `CommonSystem`). The `vtkAlgorithm` pipeline
      (`Update()`, dirty-flag propagation). Needed before any filter exists. Tracked as issue #51.

Numbers are dependency levels, not a strict sequence: items sharing a number are independent of
each other. So the real shape is `Core` → {`Math`, `System`} → {`Transforms`, `Misc`} →
`DataModel` → `ExecutionModel`, **not** "everything after `CommonCore` in parallel".

`CommonCache`, `CommonColor` and `CommonComputationalGeometry` are also `Common*` modules but are
not needed until Phase 3 — see there.

**Exit criteria**: all category-1 tests from `Common/Core/Testing/Cxx` and
`Common/DataModel/Testing/Cxx` ported and passing. Note this is a much larger body of work than
a directory count suggests — those two directories alone hold on the order of 230 pure-logic
tests (see **Sizing caveat** in the Snapshot).

## Phase 2 — IO (Legacy + XML), round-trip tests as acceptance criteria

Neither reader/writer module is reachable directly — both sit on `IOCore`, and each pulls one
more module the earlier version of this roadmap omitted:

- [ ] 1. `vtk-io-core` (`VTK::IOCore`) — `DEPENDS: CommonCore, CommonExecutionModel` (+ private
      `CommonDataModel`, `CommonMisc`, `lz4`, `lzma`, `zlib`, `utf8`, `fast_float`). Compression
      and encoding substrate for everything below.
- [ ] 2. `vtk-io-xml-parser` (`VTK::IOXMLParser`) — `DEPENDS: CommonCore, CommonDataModel`
      (+ private `IOCore`, `expat`). In Rust this is where an XML crate gets chosen instead of
      porting `expat` bindings; record the choice in `docs/decisions/`.
- [ ] 3. `vtk-io-legacy` (`VTK::IOLegacy`) — `.vtk` reader/writer. `DEPENDS: CommonCore,
      CommonDataModel, CommonExecutionModel, IOCore, IOCellGrid, nlohmannjson`.
- [ ] 3. `vtk-io-xml` (`VTK::IOXML`) — `.vtp`/`.vtu`/etc. `DEPENDS: CommonCore,
      CommonExecutionModel, IOXMLParser` (+ private `CommonDataModel`, `CommonMisc`,
      `CommonSystem`, `IOCore`).

**Open scope question — `IOCellGrid`.** `IOLegacy` depends on it, and it brings an entire
additional data model (`vtkCellGrid`) that Phase 1 does not port. Decide before starting Phase 2:
port `vtkCellGrid` too, or carve the cell-grid paths out of the legacy reader/writer and accept
a documented gap against upstream. Record the call in `docs/decisions/`.

IO is deliberately ahead of filters: its tests are self-verifying round-trips (write, read back,
compare) with no rendering or external baseline needed. That rationale holds for a *subset*, not
for the module as a whole — `IO/Legacy/vtk.module` has `TEST_DEPENDS` on `FiltersAMR`,
`FiltersGeometry`, `ImagingCore`, `InteractionStyle`, `RenderingOpenGL2`, and the test executable
is built with `RENDERING_FACTORY`. In `IO/Legacy/Testing/Cxx/CMakeLists.txt`, 5 of 11 tests are
flagged `,NO_DATA,NO_VALID` and are genuinely self-contained; those are the Phase 2 net. Triage
per test, not per module.

**Exit criteria**: round-trip tests pass against files produced by *real* upstream VTK (not
just self-produced files) — use the reference tree's build or a system VTK install to generate
fixtures once, check the fixtures into `rust/crates/vtk-io-*/tests/fixtures/`. Building that
upstream VTK is itself unscheduled work; see Phase 0.

## Phase 3 — `Filters` (Core, General, Sources, Geometry)

Four `Common*`/`Filters*` modules that no earlier phase schedules are prerequisites here. They
are private deps, so they do not show up in a naive read of the public dependency graph:

- [ ] 0. `vtk-common-cache` (`VTK::CommonCache`) — private dep of `FiltersCore` and
      `FiltersGeneral`.
- [ ] 0. `vtk-common-computational-geometry` (`VTK::CommonComputationalGeometry`) — private dep
      of `FiltersGeneral` and `FiltersSources`.
- [ ] 0. `vtk-filters-reduction` (`VTK::FiltersReduction`) — private dep of `FiltersCore`.
- [ ] 0. `vtk-filters-verdict` (`VTK::FiltersVerdict`) — private dep of `FiltersGeneral`
      (mesh-quality metrics; wraps the `verdict` third-party library upstream).

Then:

- [ ] 1. `vtk-filters-core` (`VTK::FiltersCore`) — `DEPENDS: CommonCore, CommonDataModel,
      CommonExecutionModel, CommonMisc` (+ private `CommonCache`, `CommonMath`, `CommonSystem`,
      `CommonTransforms`, `FiltersReduction`).
- [ ] 2. `vtk-filters-geometry` (`VTK::FiltersGeometry`) — `DEPENDS: CommonCore,
      CommonDataModel, CommonExecutionModel` (+ private `FiltersCore`).
- [ ] 2. `vtk-filters-general` (`VTK::FiltersGeneral`) — `DEPENDS: CommonCore, CommonDataModel,
      CommonExecutionModel, CommonMisc, FiltersCore` (+ private `CommonCache`,
      `CommonComputationalGeometry`, `CommonMath`, `CommonSystem`, `CommonTransforms`,
      `FiltersGeometry`, `FiltersVerdict`).
- [ ] 3. `vtk-filters-sources` (`VTK::FiltersSources`) — `DEPENDS: CommonDataModel,
      CommonExecutionModel` (+ private `CommonComputationalGeometry`, `CommonCore`,
      `CommonTransforms`, `FiltersCore`, `FiltersGeneral`).

These are the first algorithms (clip, decimate, contour, geometry extraction). Pipeline
execution model from Phase 1 gets exercised for real here.

Note `FiltersSources` lands *after* `FiltersGeneral`, not alongside it: it privately depends on
both `FiltersCore` and `FiltersGeneral`.

**Exit criteria**: category-1 tests pass; category-2 tests that only need `Common*`+`IO*`
fixtures (no image baseline) pass.

## Phase 4 — Rendering (core)

- [ ] Pick and record the backend decision in `docs/decisions/` — default assumption: `wgpu`,
      not a port of `RenderingOpenGL2`. `vtk-rendering-core` defines the same conceptual
      surface (`Renderer`, `RenderWindow`, `Actor`, `Mapper`) but the backend is new code, not a
      translation of `Rendering/OpenGL2`. **Read `Rendering/WebGPU` in the reference tree before
      deciding** — upstream already has a WebGPU backend, so this is closer to porting an
      existing module than to inventing one, and its structure is evidence about which
      abstractions survive the move off OpenGL.
- [ ] `vtk-rendering-core` — scene graph, camera, actor/mapper/property model.
- [ ] First category-3 tests become portable here: prefer converting pixel-comparison tests to
      data/geometry assertions (compare the mesh fed to the renderer, not the rendered pixels)
      per `AGENTS.md`; keep true pixel-diff tests as a small, explicitly-tagged minority.

## Phase 5 — Interaction / Widgets

- [ ] `vtk-interaction-style` — camera/trackball interactors. Depends on Phase 4.

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
- **Performance benchmarking**: no benchmark harness or perf-regression gate exists yet — nothing
  proves the port is "at least not slower" than upstream C++. Direction: `criterion` benches per
  crate under `benches/`, added once that crate has real code (an empty skeleton has nothing to
  benchmark). Informational only in CI, not gated, until two prerequisites exist: the FFI
  reference-testing oracle above (a real C++ baseline to diff against — without it "not slower"
  has nothing to compare to) and a dedicated, non-shared runner (perf numbers on shared CI
  runners are too noisy to gate on). Not required for Phase 0/1 bootstrap; revisit once Phase 1
  has real code and the oracle question above is resolved.

## On "port the tests first" as a methodology (see `AGENTS.md` § Testing strategy)

Works well, with caveats:

- It's a strong fit for the pure-logic tests — genuine unit tests of data structures and math,
  no framework mismatch (VTK's `int Test(int, char*[])` -> error count maps 1:1 onto `#[test]` +
  `assert!`). This is real red/green TDD, not aspirational. There are far more of them than the
  original "73 directories" figure implied; see **Sizing caveat** in the Snapshot.
- For IO and filter tests it's better described as **characterization/acceptance testing**
  than strict TDD: these are integration-shaped (build a small pipeline, run it, check the
  result), and porting them *before* the module exists mostly means writing them
  `#[ignore]`d as a spec, then implementing to satisfy them — still valuable, just not
  tiny-increment TDD.
- Image-comparison tests (`DATA{...}` sites) are the real exception: don't port them as-is
  before a renderer exists, and even then prefer converting pixel-diffs to data/geometry
  assertions where the test's actual intent allows it.
- The one addition beyond "just port the tests": keep `docs/test-mapping.csv` as a traceability
  ledger (original path -> rust path -> category -> status). Without it, "how much of VTK is
  actually ported" is unanswerable once this passes a few hundred tests, and upstream parity
  claims aren't auditable.
