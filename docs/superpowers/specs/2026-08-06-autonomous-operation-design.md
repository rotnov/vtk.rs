# Autonomous operation — design

Date: 2026-08-06
Issue: [#19](https://github.com/rotnov/vtk.rs/issues/19)
Status: approved, not yet implemented

## Problem

This project is meant to run with no human in the loop. It cannot yet. An agent starting a
session today has no way to answer *what do I do now*, no CI to tell it whether what it did was
right, no rule for when to stop, and no way to hand off to the next session except by leaving the
next agent to re-derive everything from scratch.

Separately, the documentation has outgrown its budget. `AGENTS.md` is auto-loaded in full at every
session start; at ~560 lines it costs the same tokens whether the task is porting a module or
answering a question, and its rules compete with each other for attention.

The load-bearing gap is CI. Branch protection is enabled, required status checks are empty, and
`ledger-check` is unwritten. Every gate `AGENTS.md` describes is currently fiction, so "green CI
is the review" means there is no review.

## What an autonomous session must be able to do

Start, decide, do, verify, finish, hand off, and recover. Each of the seven below is a gap today.

## Design

### 1. No session bound — an orchestrator loop that delegates

Revised from an earlier draft that bounded each session to one issue. The owner's correction:
this runs until the project is done, and work is pushed into subagents so the main session never
carries the noise of actually porting a module.

The main session becomes an **orchestrator**, never a porter:

1. `cargo xtask next` → the next unblocked issue.
2. Dispatch it to a fresh `Agent` call, one subagent per issue, with the full task context
   (issue body, module `DEPENDS`, reference-tree paths, current ledger rows) in the prompt —
   the subagent has no memory of the orchestrator's history, so the prompt must be self-contained.
3. The subagent does the actual work — reads the C++ reference, writes the Rust, ports the
   tests, iterates against local `cargo test`/`clippy`/`llvm-cov` until green, opens the PR — and
   returns only an outcome: PR number and green, or blocked with a reason.
4. The orchestrator merges on green, appends a session-log entry, and loops to step 1.
5. The loop ends when `cargo xtask next` reports nothing left — every module in every phase is
   `ported` — or when something requires the owner (see **Blocked-work policy**).

This is what actually controls context, not a session boundary. `vtk-common-core` alone is
~95 ported tests; doing that inline would blow the window regardless of any stopping rule. Pushed
into a subagent, the orchestrator's context holds only a one-line outcome per issue no matter how
large the module was.

`autoCompactWindow` is set to 250000 in `.claude/settings.json` as the backstop for the
orchestrator itself, which still accumulates one entry per completed issue across an unbounded
run and will eventually need to compact.

One issue per subagent, not one module per subagent: a module can span several issues (e.g. a
prerequisite third-party decision plus the port itself), and keeping the unit at "one issue" is
what keeps `cargo xtask next` and the ledger's granularity aligned.

### 2. Entry point: `cargo xtask next`

The single command an agent runs to start. It prints the next unblocked issue with everything
needed to begin: the issue number, the module, the module's `DEPENDS`, the reference-tree paths
for its sources and tests, and the current ledger rows for it.

Unblocked means: its milestone is the **lowest-numbered phase that still has any open, non-parked
issue** (phases complete in order, per `ROADMAP.md`), its `blocked by` issues are all closed, and
its module's dependency-level predecessors are `ported` in the ledger.

This replaces the current situation, where deciding what to do next means reading `ROADMAP.md`,
the ledger, and the GitHub issue list and reasoning over all three. Making it a command rather
than a procedure removes both the token cost and the variance.

### 3. Status is computed, never declared

`docs/test-mapping.csv` is the only source of truth for what is ported. Module status is derived
from it, not asserted anywhere.

`ROADMAP.md` loses its checkboxes — a hand-maintained copy of a computed value only diverges. It
keeps what cannot be computed: phase order, the dependency graph, rationale, open questions.

The three artifacts each answer exactly one question, with no overlap:

| artifact | question |
|---|---|
| `ROADMAP.md` | in what order, and why |
| GitHub issues / milestones | what is being worked on now |
| `docs/test-mapping.csv` | how much of VTK's suite is actually ported |

### 4. The parity gate becomes real: a fourth `ledger-check` assertion

Today "a module is done only when its ported tests are green" is prose, while coverage is
enforced by CI. Since an own test satisfies coverage just as well as a ported one, the only gate
protecting the project's purpose is the unenforced one. See lesson
[0006](../../lessons/0006-new-rule-weakened-existing-one.md).

Add a fourth assertion to `ledger-check`:

- **parity** — if a crate contains any code, its module has at least one ledger row with
  `status=ported`. A crate cannot exist on own tests alone.

`ledger-check` then asserts *exists* / *complete* / *fresh* / *parity*, and CI reports per-module
suite coverage alongside line coverage. Both numbers are computed from artifacts; neither can be
declared.

### 5. Blocked-work policy

`AGENTS.md` currently says to surface a blocker to the owner and stop. In autonomous operation
the owner is absent, so that is not a policy — it is a halt.

Instead: park it and keep moving.

1. The subagent comments on its issue with what is blocked and precisely what would unblock it,
   applies the `blocked` label, removes it from the current milestone, and returns `blocked` (not
   a crash) to the orchestrator.
2. The orchestrator records a lesson if the blocker was avoidable, and loops to the next
   unblocked issue — a blocked issue does not stop the run.

Only three things genuinely halt the whole loop rather than park one issue: a credential the
agent does not have, a licensing or scope decision, and a destructive action outside the writable
paths. Those go to the owner and the loop stops — everything else is parked and the loop
continues.

A blocked issue is not failure; an *unrecorded* blocked issue is.

### 6. Session logs, decomposed

One file per session, `docs/sessions/YYYY/MM/YYYY-MM-DD-<slug>.md`, with front matter:

```yaml
date: 2026-08-06
issues: [19]
prs: [20]
decisions: ['0004']
lessons: ['0007']
phase: 0
outcome: merged | parked | blocked
```

The body answers three questions and nothing else: what was done, what was decided and why, what
was left open. Never a transcript — the PR already holds that, and duplicating it costs tokens on
every read while adding nothing.

One entry is appended by the orchestrator per merged or parked issue, not per calendar session —
with no session boundary, "session" here means one pass of the orchestrator loop. The log exists
so a fresh orchestrator (after a restart, or a compaction) resumes from twenty lines instead of
re-deriving state. That is its whole justification; if it stops being cheaper than re-derivation,
it should be deleted.

### 7. Learning consolidates into rules, skills, or agents

Rule 1's ladder gains a rung. A lesson escalates by *what kind of thing it is*, not by severity:

| the lesson is about | it becomes | where it lives | context cost |
|---|---|---|---|
| a fact or constraint | a rule | `AGENTS.md` | every session |
| a recurring **procedure** | a skill | `.claude/skills/<name>/` | on demand |
| a recurring **role** | a subagent | `.claude/agents/<name>.md` | on demand |
| anything mechanically checkable | a CI check | `.github/workflows/` | zero |

Preference order is right to left: a check beats a skill beats a rule. A check costs no context
and cannot be rationalised around. A skill costs context only when invoked. A rule costs context
always and is merely advisory.

This is what keeps `AGENTS.md` from growing without bound while the project keeps learning —
learning moves procedure *out* of the always-loaded file rather than into it.

### 8. Token budget

`AGENTS.md` shrinks to what a machine cannot check and a skill cannot hold: the untrusted-content
rules, the autopilot dispositions, Rule 1, the language rule, the two gates stated in one line
each, and a routing table mapping task to document. Target ~150 lines against today's ~560.

Everything checkable moves to CI, which also makes it binding rather than advisory:

| rule today | becomes |
|---|---|
| don't write outside the writable paths | `paths-check` — diff against the pin outside the allowlist fails |
| 100% coverage | already a gate |
| update the ledger with the test | `ledger-check` |
| a module is done only via ported tests | `ledger-check parity` |
| everything in English | non-ASCII check over `rust/` and `docs/` — catches the real risk here, Cyrillic |
| don't push to `master` | already branch protection |

Everything procedural moves to `docs/` or a skill, reachable through the routing table.

Estimated saving is roughly 400 lines per session start, but the larger gain is not tokens: a
short list of rules that cannot be checked is read in full, while a long list of mixed rules is
skimmed.

### 9. The observer view: GitHub Pages

A static site, published from `master` by CI, that **computes everything and stores nothing**.
Introducing a fourth place where status lives would guarantee it diverges from the other three.

| panel | computed from |
|---|---|
| porting progress, % of VTK suite, by module and phase | `docs/test-mapping.csv` |
| current phase and work in flight | ledger + open issues/PRs by milestone |
| lessons: total, promoted, enforced | front matter in `docs/lessons/` |
| decisions | `docs/decisions/` |
| session history | front matter in `docs/sessions/` |
| line and function coverage | CI artifact |

The headline number is deliberately *not* module count. It is the **ratio of enforced to open
lessons** — the one figure that distinguishes a project that is learning from one that is
journalling.

Generator: a Python script under `.github/`, because `rust/` does not exist yet and creating a
Cargo workspace to render a dashboard would be the tail wagging the dog. Revisit moving it into
`cargo xtask` once the workspace exists and can share the ledger parser.

## Dependency order

CI first — until it exists, every gate above is decoration, and each subsequent piece assumes
it.

1. `.github/workflows/` with `paths-check` and the non-ASCII language check. These need no
   `rust/` and make two prose rules binding immediately.
2. `rust/` workspace skeleton, plus `cargo test` / `clippy` / `fmt` / coverage in CI.
3. `cargo xtask ledger-check` (four assertions) and `cargo xtask next`.
4. Required status checks added to branch protection. Only now is "green CI is the review" true.
5. `AGENTS.md` shrunk, procedures moved out, routing table added.
6. Session logs and the blocked-work policy.
7. GitHub Pages.

## Out of scope

Porting any VTK module. This spec builds the machine that ports them; the first port is the
first thing that tests it.

## What would falsify this design

- If `cargo xtask next` is not enough to start a subagent — if the orchestrator still reads
  `ROADMAP.md` and the ledger by hand to decide — the entry point is not doing its job.
- If the orchestrator's own context grows unmanageably despite delegation, work is leaking into
  it that belongs in the subagent (e.g. reading source files directly instead of trusting the
  subagent's returned outcome).
- If `AGENTS.md` grows past ~200 lines again, the ladder in §7 is not being used and lessons are
  being written as rules when they should be checks or skills.
- If the enforced-to-open lesson ratio does not rise over time, Rule 1 is journalling.
