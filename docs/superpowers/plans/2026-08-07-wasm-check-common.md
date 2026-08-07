# Wire `cargo check --target wasm32-unknown-unknown` for `Common*` Crates Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close the tracked gap in `ROADMAP.md` § Phase 0: add a `cargo-check-wasm32` CI job that
runs `cargo check --target wasm32-unknown-unknown` across the 7 existing `vtk-common-*` crates,
proven by a positive-control smoke test, and flip the ROADMAP checkbox.

**Architecture:** A fourth job in the existing `.github/workflows/rust-checks.yml`, following the
exact shape of the three jobs already there (`cargo-test`/`cargo-clippy`/`cargo-fmt`): checkout,
install the Rust toolchain with the `wasm32-unknown-unknown` target added, restore the Cargo
cache, run one `cargo check` invocation with explicit `-p` flags naming every crate in scope.
Explicit `-p` flags, not `--workspace`, because `IO*` crates (not yet ported) are excluded by
design per `AGENTS.md` § WebAssembly — a wildcard would silently include them later and break.

**Tech Stack:** Rust stable, Cargo workspaces, `dtolnay/rust-toolchain` and `Swatinem/rust-cache`
GitHub Actions (same as the existing three jobs).

## Global Constraints

- Scope is exactly the 7 crates in `rust/Cargo.toml`'s `members` list today: `vtk-common-core`,
  `vtk-common-math`, `vtk-common-system`, `vtk-common-transforms`, `vtk-common-misc`,
  `vtk-common-data-model`, `vtk-common-execution-model`. `Filters*` crates don't exist yet.
- Requirement source, verbatim (`AGENTS.md` § WebAssembly): "`Common*` and `Filters*` must compile
  for `wasm32-unknown-unknown`. Check it in CI (`cargo check --target wasm32-unknown-unknown`) from
  the moment those crates exist." Build check only — running tests under wasm is explicitly
  deferred in the same section ("Running the test suite *under* wasm is a separate, heavier
  problem... Deferred — the build check is the constraint that matters now.").
- `IO*` crates are explicitly exempted from this requirement (`AGENTS.md` § WebAssembly): "not
  expected to build for `wasm32-unknown-unknown` as-is." Do not add them to this job's `-p` list
  now or in the future without first making them wasm-compatible.
- Verified empirically in this planning session (not asserted): all 7 crates in scope currently
  `cargo check --target wasm32-unknown-unknown` clean with zero errors — this is a new CI job that
  starts green, not one that requires code changes to pass.
- Writable paths: `.github/workflows/`, `AGENTS.md`, `ROADMAP.md` (`AGENTS.md` § What is writable).
  This plan touches no crate source.
- Out of scope (per issue [#36](https://github.com/rotnov/vtk.rs/issues/36) and
  `ROADMAP.md` § Phase 0): `Filters*` crates (don't exist yet — add them to this job's `-p` list in
  the PR that creates them, this plan does not attempt to future-proof the list against crates that
  don't exist), running the test suite under wasm, and marking this check `required` in branch
  protection (dependency-order Step 4, bundled with the other required-checks wiring — see
  `docs/superpowers/specs/2026-08-06-autonomous-operation-design.md` § Dependency order).

---

### Task 1: `cargo-check-wasm32` CI job

**Files:**
- Modify: `.github/workflows/rust-checks.yml` (add a fourth job)
- Modify: `AGENTS.md:176-178` (§ Required checks — flip from "not yet wired" to "live")

**Interfaces:**
- Consumes: the `rust/` workspace and its 7 crates (already exist, no changes needed).
- Produces: a `cargo-check-wasm32` job in `.github/workflows/rust-checks.yml`, which the
  controller-executed smoke test after this task exercises directly (no code interface — the next
  step reads the job by name from the workflow file).

- [ ] **Step 1: Add the `cargo-check-wasm32` job**

Current full content of `.github/workflows/rust-checks.yml`:

```yaml
name: rust-checks

on:
  pull_request:
    types: [opened, synchronize, reopened, labeled]
    branches: [master]

jobs:
  cargo-test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - uses: dtolnay/rust-toolchain@stable

      - uses: Swatinem/rust-cache@v2
        with:
          workspaces: rust

      - name: cargo test
        working-directory: rust
        run: cargo test --workspace --all-features

  cargo-clippy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy

      - uses: Swatinem/rust-cache@v2
        with:
          workspaces: rust

      - name: cargo clippy
        working-directory: rust
        run: cargo clippy --workspace --all-targets --all-features -- -D warnings

  cargo-fmt:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt

      - name: cargo fmt --check
        working-directory: rust
        run: cargo fmt --all --check
```

Append a fourth job after `cargo-fmt`, keeping the blank-line-between-jobs style:

```yaml

  cargo-check-wasm32:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: wasm32-unknown-unknown

      - uses: Swatinem/rust-cache@v2
        with:
          workspaces: rust

      # Explicit crate list, not --workspace: IO* crates are exempt from the wasm build
      # requirement per AGENTS.md § WebAssembly, so a wildcard would silently pull them in
      # once they're ported. Add new Common*/Filters* crates to this list when they're created.
      - name: cargo check --target wasm32-unknown-unknown
        working-directory: rust
        run: |
          cargo check --target wasm32-unknown-unknown \
            -p vtk-common-core \
            -p vtk-common-math \
            -p vtk-common-system \
            -p vtk-common-transforms \
            -p vtk-common-misc \
            -p vtk-common-data-model \
            -p vtk-common-execution-model
```

The full file after this step has four jobs: `cargo-test`, `cargo-clippy`, `cargo-fmt`,
`cargo-check-wasm32`, each separated by one blank line, matching the existing style exactly.

- [ ] **Step 2: Verify locally**

From `rust/`:

```bash
rustup target add wasm32-unknown-unknown
cargo check --target wasm32-unknown-unknown \
  -p vtk-common-core \
  -p vtk-common-math \
  -p vtk-common-system \
  -p vtk-common-transforms \
  -p vtk-common-misc \
  -p vtk-common-data-model \
  -p vtk-common-execution-model
```

Expected: exits `0`, `Finished` for all 7 crates, no errors. (Confirmed to pass as of this plan's
writing — every crate is a doc-comment-only stub with no code that could fail.)

- [ ] **Step 3: Update `AGENTS.md` § Required checks**

Current text at `AGENTS.md:176-178`:

```markdown
- `cargo check --target wasm32-unknown-unknown` for `Common*`/`Filters*` — see **WebAssembly**.
  Not yet wired even though the trigger condition (those crates existing) has fired — tracked as
  a known gap in `ROADMAP.md` § Phase 0, not a silent omission.
```

Replace with:

```markdown
- `cargo check --target wasm32-unknown-unknown` for `Common*`/`Filters*` — see **WebAssembly**.
  Live today (`cargo-check-wasm32` in `.github/workflows/rust-checks.yml`). The crate list is
  explicit `-p` flags, not a wildcard — add new `Common*`/`Filters*` crates to the job when they're
  created. `IO*` crates are excluded by design, see **WebAssembly**.
```

Do not change any other bullet in § Required checks (the `cargo xtask ledger-check` and coverage
bullets remain "not yet wired" — those are separate, later steps).

- [ ] **Step 4: Commit**

```bash
git add .github/workflows/rust-checks.yml AGENTS.md
git commit -m "feat: wire cargo check --target wasm32-unknown-unknown for Common* crates"
```

---

## After the task: positive-control smoke test (controller-executed, not a task)

Per established precedent (`docs/superpowers/plans/2026-08-07-rust-workspace-ci.md`'s smoke-test
section, and `docs/superpowers/plans/2026-08-07-paths-and-language-checks.md` Task 7 Step 5), a new
CI check needs proof it actually fires red on a real violation before it's trusted. The controller
(not a dispatched subagent) does this directly after Task 1 is merged to this plan's branch, using
a disposable branch that is never merged:

1. `git checkout -b smoke/wasm-check-red <this-plan's-branch>`, push it, and open one throwaway PR
   from `smoke/wasm-check-red` against `master` — this is what gives the branch CI runs to read.
   Do not merge it.
2. Add this trigger to `rust/crates/vtk-common-core/src/lib.rs` (verified empirically during
   planning: compiles clean on the native target, fails with `E0433`/`E0599` under
   `wasm32-unknown-unknown` because `std::os::unix` does not exist outside `unix`/`redox` targets):

   ```rust
   use std::os::unix::fs::PermissionsExt;

   pub fn mode_of(p: std::fs::Permissions) -> u32 {
       p.mode()
   }
   ```

   Push. Confirm this run shows `cargo-check-wasm32` red and `cargo-test`/`cargo-clippy`/
   `cargo-fmt` green (`cargo-test`/`cargo-clippy` compile and lint on the native target only, where
   this code is valid; `cargo-fmt` only checks formatting, not compilation). This isolates the new
   job exactly as the Step 2 plan's three-job smoke test isolated each of its jobs.
3. Revert the trigger commit, push the revert. Confirm the next run is green on all four jobs.
4. Close the throwaway PR without merging and delete the branch
   (`git push origin --delete smoke/wasm-check-red`).
5. Confirm this plan's real PR (Task 1 only, no smoke changes) is clean on all four jobs.

Record the CI run URLs and pass/fail table in this plan document (mirroring the table at the top of
`docs/superpowers/plans/2026-08-07-rust-workspace-ci.md`) once run, since the disposable branch that
produced them will no longer exist.

### Smoke-test results (recorded after the run; `smoke/wasm-check-red` and PR #37 no longer exist)

Trigger run (`std::os::unix::fs::PermissionsExt` added to `vtk-common-core`), commit `c9ba078751`:

| Job | Result | Run |
| --- | --- | --- |
| `cargo-check-wasm32` | failure (expected) | [31185265110](https://github.com/rotnov/vtk.rs/actions/runs/31185265110/job/92888110904) |
| `cargo-clippy` | success | [31185265110](https://github.com/rotnov/vtk.rs/actions/runs/31185265110/job/92888110897) |
| `cargo-fmt` | success | [31185265110](https://github.com/rotnov/vtk.rs/actions/runs/31185265110/job/92888111142) |
| `cargo-test` | success | [31185265110](https://github.com/rotnov/vtk.rs/actions/runs/31185265110/job/92888110824) |
| `language-check` | success | [31185268378](https://github.com/rotnov/vtk.rs/actions/runs/31185268378/job/92888117434) |
| `paths-check` | success | [31185268378](https://github.com/rotnov/vtk.rs/actions/runs/31185268378/job/92888117555) |

Revert run (commit `d9f27f14c3`), confirming the job returns to green once the violation is gone:

| Job | Result | Run |
| --- | --- | --- |
| `cargo-check-wasm32` | success | [31185700247](https://github.com/rotnov/vtk.rs/actions/runs/31185700247/job/92889546641) |
| `cargo-clippy` | success | [31185700247](https://github.com/rotnov/vtk.rs/actions/runs/31185700247/job/92889546674) |
| `cargo-fmt` | success | [31185700247](https://github.com/rotnov/vtk.rs/actions/runs/31185700247/job/92889546926) |
| `cargo-test` | success | [31185700247](https://github.com/rotnov/vtk.rs/actions/runs/31185700247/job/92889546729) |
| `language-check` | success | [31185700483](https://github.com/rotnov/vtk.rs/actions/runs/31185700483/job/92889546611) |
| `paths-check` | success | [31185700483](https://github.com/rotnov/vtk.rs/actions/runs/31185700483/job/92889546484) |

The new job isolates exactly as designed: it alone went red on a real wasm-incompatible API and
alone returned to green once that API use was removed, while the three native-target jobs
(`cargo-test`/`cargo-clippy`/`cargo-fmt`) and the two unrelated checks (`language-check`/
`paths-check`) stayed green throughout, confirming no cross-job coupling.

## After the smoke test: flip the ROADMAP checkbox

Current text at `ROADMAP.md:66-71`:

```markdown
- [ ] `cargo check --target wasm32-unknown-unknown` wired into CI for `Common*`/`Filters*`
      crates, per `AGENTS.md` § WebAssembly ("from the moment those crates exist"). The trigger
      condition already fired — the 7 `Common*` skeletons above exist — so this is a known,
      tracked gap rather than a future deferral: close it in a dedicated small PR (with its own
      positive-control smoke test, per this repo's convention) before Phase 1 implementation
      work begins in earnest.
```

Replace with:

```markdown
- [x] `cargo check --target wasm32-unknown-unknown` wired into CI for `Common*`/`Filters*`
      crates, per `AGENTS.md` § WebAssembly — done: `cargo-check-wasm32` job in
      `.github/workflows/rust-checks.yml`, covering all 7 `vtk-common-*` crates via explicit `-p`
      flags (`Filters*` don't exist yet — add them to the job when they do), proven by a
      positive-control smoke test to fail on a real wasm-incompatible API — see
      `docs/superpowers/plans/2026-08-07-wasm-check-common.md`.
```

Commit this on the same plan branch, in the same PR, once the smoke test above has confirmed the
job fires red and the branch is back to all-green:

```bash
git add ROADMAP.md
git commit -m "docs: mark wasm32 CI check gap closed in ROADMAP"
```
