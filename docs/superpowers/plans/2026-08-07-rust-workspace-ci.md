# Rust Workspace Skeleton + CI (Dependency-order Step 2) Implementation Plan

> **Status: implemented and smoke-tested, pending merge via PR
> [#33](https://github.com/rotnov/vtk.rs/pull/33)** (closes
> [#32](https://github.com/rotnov/vtk.rs/issues/32)).
>
> The positive-control smoke-test sequence below ran on disposable branch `smoke/rust-checks-red`,
> throwaway PR [#34](https://github.com/rotnov/vtk.rs/pull/34) (never merged, branch deleted after
> use). While running it, the originally-specified `cargo-test` trigger (`assert!(false)`) was
> found to also fail `cargo-clippy` — see
> `docs/lessons/0011-smoke-trigger-clippy-claim-never-run.md` — and was corrected to
> `assert_eq!(1, 2)` before the run recorded below. All three runs are recorded here because the
> disposable branch that produced them no longer exists:
>
> | trigger | cargo-test | cargo-clippy | cargo-fmt | CI run |
> |---|---|---|---|---|
> | `assert_eq!(1, 2)` in a `#[test]` | failure | success | success | [31180976476](https://github.com/rotnov/vtk.rs/actions/runs/31180976476) |
> | unused `use std::collections::HashMap;` | success | failure | success | [31181194673](https://github.com/rotnov/vtk.rs/actions/runs/31181194673) |
> | `pub fn x (  ) { }` | success | success | failure | [31181603385](https://github.com/rotnov/vtk.rs/actions/runs/31181603385) |
>
> Each run independently proves exactly one job goes red on a real violation while the other two
> stay green. All three jobs are now proven to fire, not just to pass.

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Stand up the `rust/` Cargo workspace skeleton (empty crates for every Phase 1 module)
and wire `cargo test` / `cargo clippy -D warnings` / `cargo fmt --check` into CI as three separate
required-check jobs.

**Architecture:** A single Cargo workspace at `rust/` with one crate per Phase 1 module, wired
with intra-workspace path dependencies that mirror each module's `DEPENDS` in `ROADMAP.md`. A new
`.github/workflows/rust-checks.yml` runs the three checks as independent jobs on every PR,
mirroring the trigger and job-per-check shape of the existing `.github/workflows/repo-checks.yml`.

**Tech Stack:** Rust stable, Cargo workspaces, `dtolnay/rust-toolchain` and `Swatinem/rust-cache`
GitHub Actions.

## Global Constraints

- Crate naming: `vtk-<kebab-case-module-path>`, mirroring the VTK module's `NAME` (`AGENTS.md` §
  Rust workspace conventions).
- A crate's `Cargo.toml` dependencies must mirror that module's `DEPENDS` in `ROADMAP.md` Phase 1
  — never introduce a dependency VTK's own module graph doesn't have.
- Required-check commands, verbatim (`AGENTS.md` § Required checks):
  `cargo test --workspace --all-features`,
  `cargo clippy --workspace --all-targets --all-features -- -D warnings`,
  `cargo fmt --all --check`.
- **The coverage gate is explicitly OUT of scope for this plan.** Verified empirically (dry run,
  2026-08-07): `cargo llvm-cov --workspace --all-features --fail-under-lines 100
  --fail-under-functions 100` hard-errors with `no coverage data found` on a workspace with zero
  executing tests — it does not report 100%. Per `docs/decisions/0001-test-coverage-metric.md`'s
  2026-08-07 amendment and `ROADMAP.md` Phase 0, the coverage job is wired in the PR that adds
  Phase 1's first crate with an actually-executing test, not in this plan. Do not add a
  `cargo-coverage` (or similarly named) CI job in this plan, and do not add a test to any crate
  purely to make such a job pass — every crate in this plan ships with zero tests, by design.
- Out of scope (per `docs/superpowers/specs/2026-08-06-autonomous-operation-design.md` §
  Dependency order): `cargo xtask ledger-check`/`next` (Step 3), the `wasm32-unknown-unknown`
  target check (not part of Step 2), branch-protection required-checks wiring (Step 4), and
  porting any VTK module — every crate's `src/lib.rs` in this plan is a doc comment only.
- Writable paths: `rust/`, `.github/workflows/`, `AGENTS.md` (`AGENTS.md` § What is writable).
  Nothing in this plan touches the read-only vendored VTK tree.
- Workspace-package fields (`version`, `edition`, `publish`) are set once in the root
  `[workspace.package]` table and inherited via `version.workspace = true` etc. in every crate —
  do not repeat literal values per crate.

---

### Task 1: `rust/` Cargo workspace skeleton — 7 empty Phase 1 crates

**Files:**
- Create: `rust/Cargo.toml`
- Create: `rust/rust-toolchain.toml`
- Create: `rust/.gitignore`
- Create: `rust/crates/vtk-common-core/Cargo.toml`
- Create: `rust/crates/vtk-common-core/src/lib.rs`
- Create: `rust/crates/vtk-common-math/Cargo.toml`
- Create: `rust/crates/vtk-common-math/src/lib.rs`
- Create: `rust/crates/vtk-common-system/Cargo.toml`
- Create: `rust/crates/vtk-common-system/src/lib.rs`
- Create: `rust/crates/vtk-common-transforms/Cargo.toml`
- Create: `rust/crates/vtk-common-transforms/src/lib.rs`
- Create: `rust/crates/vtk-common-misc/Cargo.toml`
- Create: `rust/crates/vtk-common-misc/src/lib.rs`
- Create: `rust/crates/vtk-common-data-model/Cargo.toml`
- Create: `rust/crates/vtk-common-data-model/src/lib.rs`
- Create: `rust/crates/vtk-common-execution-model/Cargo.toml`
- Create: `rust/crates/vtk-common-execution-model/src/lib.rs`

**Interfaces:**
- Produces: 7 workspace member crates at the paths above, importable by later Phase 1 porting
  work via `{ path = "../<crate-name>" }`. Task 2 consumes the workspace root at `rust/` (it runs
  `cargo <subcommand>` with `working-directory: rust`) but touches none of these files.

- [ ] **Step 1: Create the workspace root manifest**

`rust/Cargo.toml`:

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
]

[workspace.package]
version = "0.1.0"
edition = "2024"
publish = false
```

- [ ] **Step 2: Pin the toolchain and ignore build output**

`rust/rust-toolchain.toml`:

```toml
[toolchain]
channel = "stable"
components = ["clippy", "rustfmt"]
```

`rust/.gitignore`:

```
/target
```

- [ ] **Step 3: Create `vtk-common-core` (no intra-workspace deps — the true root)**

`rust/crates/vtk-common-core/Cargo.toml`:

```toml
[package]
name = "vtk-common-core"
version.workspace = true
edition.workspace = true
publish.workspace = true
description = "Port of VTK::CommonCore."
```

`rust/crates/vtk-common-core/src/lib.rs`:

```rust
//! Port of VTK::CommonCore. See ROADMAP.md Phase 1 for scope and dependency order.
```

- [ ] **Step 4: Create `vtk-common-math` (`DEPENDS: CommonCore`)**

`rust/crates/vtk-common-math/Cargo.toml`:

```toml
[package]
name = "vtk-common-math"
version.workspace = true
edition.workspace = true
publish.workspace = true
description = "Port of VTK::CommonMath."

[dependencies]
vtk-common-core = { path = "../vtk-common-core" }
```

`rust/crates/vtk-common-math/src/lib.rs`:

```rust
//! Port of VTK::CommonMath. See ROADMAP.md Phase 1 for scope and dependency order.
```

- [ ] **Step 5: Create `vtk-common-system` (`DEPENDS: CommonCore`)**

`rust/crates/vtk-common-system/Cargo.toml`:

```toml
[package]
name = "vtk-common-system"
version.workspace = true
edition.workspace = true
publish.workspace = true
description = "Port of VTK::CommonSystem."

[dependencies]
vtk-common-core = { path = "../vtk-common-core" }
```

`rust/crates/vtk-common-system/src/lib.rs`:

```rust
//! Port of VTK::CommonSystem. See ROADMAP.md Phase 1 for scope and dependency order.
```

- [ ] **Step 6: Create `vtk-common-transforms` (`DEPENDS: CommonCore, CommonMath`)**

`rust/crates/vtk-common-transforms/Cargo.toml`:

```toml
[package]
name = "vtk-common-transforms"
version.workspace = true
edition.workspace = true
publish.workspace = true
description = "Port of VTK::CommonTransforms."

[dependencies]
vtk-common-core = { path = "../vtk-common-core" }
vtk-common-math = { path = "../vtk-common-math" }
```

`rust/crates/vtk-common-transforms/src/lib.rs`:

```rust
//! Port of VTK::CommonTransforms. See ROADMAP.md Phase 1 for scope and dependency order.
```

- [ ] **Step 7: Create `vtk-common-misc` (`DEPENDS: CommonCore, CommonMath`)**

`rust/crates/vtk-common-misc/Cargo.toml`:

```toml
[package]
name = "vtk-common-misc"
version.workspace = true
edition.workspace = true
publish.workspace = true
description = "Port of VTK::CommonMisc."

[dependencies]
vtk-common-core = { path = "../vtk-common-core" }
vtk-common-math = { path = "../vtk-common-math" }
```

`rust/crates/vtk-common-misc/src/lib.rs`:

```rust
//! Port of VTK::CommonMisc. See ROADMAP.md Phase 1 for scope and dependency order.
```

- [ ] **Step 8: Create `vtk-common-data-model` (`DEPENDS: CommonCore, CommonMath,
      CommonTransforms` + private `CommonMisc`, `CommonSystem`)**

`rust/crates/vtk-common-data-model/Cargo.toml`:

```toml
[package]
name = "vtk-common-data-model"
version.workspace = true
edition.workspace = true
publish.workspace = true
description = "Port of VTK::CommonDataModel."

[dependencies]
vtk-common-core = { path = "../vtk-common-core" }
vtk-common-math = { path = "../vtk-common-math" }
vtk-common-transforms = { path = "../vtk-common-transforms" }
vtk-common-misc = { path = "../vtk-common-misc" }
vtk-common-system = { path = "../vtk-common-system" }
```

`rust/crates/vtk-common-data-model/src/lib.rs`:

```rust
//! Port of VTK::CommonDataModel. See ROADMAP.md Phase 1 for scope and dependency order.
```

- [ ] **Step 9: Create `vtk-common-execution-model` (`DEPENDS: CommonCore, CommonDataModel` +
      private `CommonMisc`, `CommonSystem`)**

`rust/crates/vtk-common-execution-model/Cargo.toml`:

```toml
[package]
name = "vtk-common-execution-model"
version.workspace = true
edition.workspace = true
publish.workspace = true
description = "Port of VTK::CommonExecutionModel."

[dependencies]
vtk-common-core = { path = "../vtk-common-core" }
vtk-common-data-model = { path = "../vtk-common-data-model" }
vtk-common-misc = { path = "../vtk-common-misc" }
vtk-common-system = { path = "../vtk-common-system" }
```

`rust/crates/vtk-common-execution-model/src/lib.rs`:

```rust
//! Port of VTK::CommonExecutionModel. See ROADMAP.md Phase 1 for scope and dependency order.
```

- [ ] **Step 10: Verify the workspace builds, tests, lints, and formats clean**

Run, from `rust/`:

```bash
cargo build --workspace
cargo test --workspace --all-features
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

Expected: all four exit `0`. `cargo test` reports `0 tests` for every crate (this is expected and
correct — no crate has any code or tests yet; do not add one to make this look more populated).

- [ ] **Step 11: Commit**

```bash
git add rust/
git commit -m "feat: rust/ Cargo workspace skeleton for Phase 1 modules"
```

---

### Task 2: Wire `cargo test`/`clippy`/`fmt` into CI, document real commands

**Files:**
- Create: `.github/workflows/rust-checks.yml`
- Modify: `AGENTS.md:576-580` (§ Commands)

**Interfaces:**
- Consumes: the `rust/` workspace from Task 1 — `working-directory: rust` in every job assumes
  Task 1's `rust/Cargo.toml` exists at that path.
- Produces: three CI jobs (`cargo-test`, `cargo-clippy`, `cargo-fmt`) that later plans/PRs can
  point to when their required-check dependencies are discussed.

- [ ] **Step 1: Create the CI workflow**

`.github/workflows/rust-checks.yml`:

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

- [ ] **Step 2: Replace the § Commands placeholder in `AGENTS.md`**

Current text at `AGENTS.md:576-580`:

```markdown
## Commands

Not yet bootstrapped — first agent to touch `rust/` should set up the Cargo workspace and
replace this section with real `cargo build` / `cargo test` / `cargo xtask test-mapping-report`
commands.
```

Replace with:

```markdown
## Commands

All commands run from `rust/`.

```bash
cargo build --workspace
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo fmt --all --check
```

`cargo xtask` commands (`ledger-check`, `test-mapping-report`, `upstream-diff`) don't exist yet —
they are dependency-order Step 3, see `docs/superpowers/specs/2026-08-06-autonomous-operation-design.md`
§ Dependency order.
```

(Match the file's existing heading level and surrounding blank-line style; do not change the
`## Do / Don't` heading that immediately follows.)

- [ ] **Step 3: Commit**

```bash
git add .github/workflows/rust-checks.yml AGENTS.md
git commit -m "feat: wire cargo test/clippy/fmt into CI as required checks"
```

---

## After both tasks: positive-control smoke tests (controller-executed, not a task)

Per established precedent (`docs/superpowers/plans/2026-08-07-paths-and-language-checks.md` Task
7 Step 5, and lessons 0007-0009 on subagent/controller drift), each new CI check needs proof it
actually fires red on a real violation — a green check that has never been seen red is not
verified, it's assumed. The controller (not a dispatched subagent) does this directly after both
tasks above are merged to this plan's branch, using a disposable branch that is never merged:

1. `git checkout -b smoke/rust-checks-red <this-plan's-branch>`, push it, and open one throwaway
   PR from `smoke/rust-checks-red` against `master` — this is what gives the branch CI runs to
   read. Do not merge it.
2. Break each check in turn as three separate commits pushed one at a time to this same branch.
   Each push starts a new CI run on the open PR; wait for that run to finish, read its three job
   results, then revert the break in a follow-up commit before pushing the next break — so each
   run is judged on its own, not conflated with the others:
   - Commit A: add `#[test] fn smoke_fails() { assert_eq!(1, 2); }` to any crate's `src/lib.rs`.
     Do not use `assert!(false)` — verified empirically (2026-08-07) that it also fails
     `cargo-clippy` (`clippy::assertions_on_constants` fires on a literal-boolean `assert!` and is
     denied under `-D warnings`), so it does not isolate `cargo-test` alone. `assert_eq!(1, 2)` is
     a runtime comparison, not a constant-literal assertion, and was verified clean under clippy.
     Push. Confirm this run shows `cargo-test` red and `cargo-clippy`/`cargo-fmt` green. Revert,
     push the revert.
   - Commit B: add `use std::collections::HashMap;` to any crate's `src/lib.rs` and never
     reference `HashMap` — an unused import fires `unused_imports`, denied under `-D warnings`.
     Push. Confirm this run shows `cargo-clippy` red and `cargo-test`/`cargo-fmt` green. Revert,
     push the revert.
   - Commit C: add `pub fn x (  ) { }` (wrong spacing `rustfmt` always normalizes) to any crate's
     `src/lib.rs`. Push. Confirm this run shows `cargo-fmt` red and `cargo-test`/`cargo-clippy`
     green. Revert, push the revert.
3. Close the throwaway PR without merging and delete the branch
   (`git push origin --delete smoke/rust-checks-red`) — three CI runs have now each independently
   proven one job goes red on a real violation while the other two stay green.
4. Confirm this plan's real PR (Task 1 + Task 2, no smoke changes) is clean on all three jobs.

Record the outcome in a lesson or as a closed line item in the ledger — this step is what turns
"the workflow file looks right" into "the workflow file was seen catching a real violation,"
mirroring how `paths-check`/`language-check` were verified.
