# 0004 — Numeric-array storage strategy for `vtk-common-core`

Status: accepted
Date: 2026-08-08

## Context

`ROADMAP.md` blocks Phase 1's first work item (issue #45: `vtkMath`, `vtkPoints`, the `vtkDataArray`
family, object/array base types) on this decision (issue #43), calling it out explicitly as
determining a lot downstream. Everything else in Phase 1 sits on top of whatever this ADR decides,
so getting it wrong is expensive to unwind later — every other module holds arrays through
whatever handle type this document settles on.

The concrete problem: `vtkDataArray` is an abstract base class with thirteen concrete numeric
subclasses (`vtkFloatArray`, `vtkDoubleArray`, `vtkIntArray`, ...). Code that doesn't know an
array's element type at compile time reaches it through `vtkAbstractArray*`/`vtkDataArray*` and
dispatches on a runtime type tag (`GetDataType()`) via `vtkTemplateMacro`
(`Common/Core/vtkSetGet.h`) — a macro that switches on the tag, casts the array's `void*` buffer to
the matching `T*` once, and calls a templated worker function. Rust has no direct analogue of
"abstract pointer plus runtime-typed template instantiation," so the port needs its own answer for:

- how a single Rust type represents "an array of numeric type unknown until runtime,"
- how the `vtkTemplateMacro` dispatch-to-generic-code pattern maps to Rust idioms,
- how VTK's actual object-identity model (many `vtkSmartPointer<vtkDataArray>` handles can point at
  the *same* mutable array — `output->SetPoints(input->GetPoints())` shares the object, it does not
  copy it) is preserved, since some ported tests may depend on that aliasing behaviour,
- whether the result is expected to be faster than the C++ it replaces, or merely as fast — the
  project's stated motivation for the port includes performance, and this is the first decision
  where "the port is safe but slower" and "the port is safe and comparable" fork.

Scope for this v0 decision, agreed during brainstorming: cover only the numeric array family
(`vtkDataArray` and its 13 `vtkTemplateMacro`-covered subclasses) in the array-of-structs (AoS)
layout VTK's filters actually consume. `vtkSOADataArrayTemplate` (struct-of-arrays), `vtkBitArray`
(packed bits, outside `vtkTemplateMacro` entirely), and non-numeric `vtkAbstractArray` subclasses
(`vtkStringArray` and similar) are deliberately out of scope — each is a smaller, separate decision
for whenever it's actually needed.

## Decision

### One concrete enum type, not a generic parameter

`DataArray` is a single enum with one variant per supported element type. Every function signature
in the port that handles "an array of runtime-unknown numeric type" names this one concrete type —
mirroring how `vtkSmartPointer<vtkDataArray>` flows through C++ signatures today without a template
parameter anywhere in sight. The alternative (a generic `TypedArray<T>` with a shared trait) still
needs a `Box<dyn Trait>` or an enum at every point code holds a heterogeneous collection (e.g. a
point-data container holding arrays of different concrete types) — it does not remove the enum, it
just moves it to a different layer, at the cost of two representations of "the same array" existing
in the codebase depending on context.

### Fixed-width variants, not a literal mirror of `vtkTemplateMacro`

`vtkTemplateMacro` covers 13 C++ types, three of which are platform-dependent (`long`/`unsigned
long` are 64-bit on Linux/macOS, 32-bit on Windows; `char` and `signed char` are both bit-for-bit
`i8` in Rust, with no equivalent ambiguity to preserve). `DataArray` instead has ten variants, one
per fixed-width Rust numeric type: `F32`, `F64`, `I8`, `U8`, `I16`, `U16`, `I32`, `U32`, `I64`,
`U64`. This does not map 1:1 onto `vtkTemplateMacro`'s case list, but it covers every bit width that
actually appears in Common/Core's tests, without carrying platform-dependent ambiguity into Rust
code that has no reason to inherit it.

`vtkIdType` (the index type used throughout VTK for point/cell/element counts) is fixed to `i64`,
unconditionally — not mirrored as a build-time-configurable width the way upstream's
`VTK_USE_64BIT_IDS` CMake flag does it. Modern VTK defaults to 64-bit IDs; fixing it removes a
combinatorial axis (two ID widths x ten element types) for a configuration this project has no
current need to support. `vtkIdType` reuses the existing `I64` variant — no separate enum case.

### Storage: `Arc<RwLock<Vec<T>>>` per variant — shared, mutable, thread-safe

Each variant wraps `Arc<RwLock<Vec<T>>>`, not a bare `Vec<T>`. This one choice resolves three
requirements simultaneously that a bare owned `Vec<T>` cannot:

- **Object identity matching VTK.** Cloning a `DataArray` clones the `Arc` (and the enum tag) — an
  O(1) pointer copy, not a deep copy of the buffer. Two `DataArray` values can name the same
  underlying storage, exactly as two `vtkSmartPointer<vtkDataArray>` can today. Mutating through
  one is visible through the other. This was chosen deliberately over a copy-on-write scheme
  (`Arc<[T]>`, detaching silently on first mutation) specifically to avoid introducing behaviour
  that diverges from what a ported test's C++ original actually does when two handles alias one
  array.
- **Zero-copy import, without a lifetime parameter.** An externally-owned buffer (e.g. `mmap`'d
  file data, relevant from Phase 2's IO modules onward) is imported by wrapping it in the same
  `Arc<RwLock<Vec<T>>>` machinery, rather than requiring `DataArray<'a>` to carry a lifetime that
  would otherwise propagate into every signature that touches an array — undoing the "one concrete
  type everywhere" property above. No `unsafe` raw-pointer aliasing is needed to get this.
- **Thread safety from the start.** `RwLock` over `RefCell` — chosen even though VTK's own
  multi-threading story (`vtkMultiThreader`) has no corresponding ROADMAP entry yet — because
  migrating a single-threaded interior-mutability choice to a thread-safe one later is an
  architectural change (every call site's error handling and lock-acquisition pattern changes),
  while the reverse is not needed. Paying the (small) extra lock cost now avoids a second migration
  later.

Component (tuple) layout is stored as flat, contiguous per-variant storage plus an
`n_components: usize` field, matching `vtkAOSDataArrayTemplate`'s native layout — the one VTK's
filters are actually written against. `vtkPoints` is a thin wrapper over one `DataArray` with
`n_components == 3`.

### Dispatch: match once per call, not once per element

The Rust translation of `vtkTemplateMacro` is a `match` on the `DataArray` enum tag, taken once at
the entry point of an algorithm, whose each arm acquires the `RwLock` once and calls an ordinary
generic function operating on the resulting `&[T]`/`&mut [T]`:

```rust
fn kernel<T: /* ... */>(data: &[T]) -> /* ... */ { /* tight loop, monomorphized per T */ }

match &array {
    DataArray::F64(buf) => kernel(&buf.read().unwrap()),
    DataArray::F32(buf) => kernel(&buf.read().unwrap()),
    // ...
}
```

This is a direct structural match for what `vtkTemplateMacro` itself does in C++ — cast once, then
run a template instantiation over the whole buffer — not a per-element dispatch. Preserving this
shape is a hard requirement, not a style preference: a `match` or lock acquisition inside the inner
loop would reintroduce per-element overhead the C++ original never pays, undermining the entire
performance rationale for the port before it is even tested.

### Validation is required before this design is trusted, not assumed

This ADR is a considered bet, not a proven one. Before it is relied on beyond `vtk-common-core`,
a synthetic microbenchmark is required — a representative kernel (e.g. a transform/reduction over
`vtkPoints`-shaped coordinate data) run both through this design and through an equivalent,
conservatively-optimized hand-written C++ reference (not literally the VTK source tree, which
carries virtual-dispatch and object-model overhead this design doesn't need to reproduce to answer
the question this ADR is actually asking) — measured with
`gungraun` (the actively maintained successor to `iai-callgrind`, which rebranded and stopped
receiving releases under its old name; deterministic instruction/cache-miss counts, not wall-clock,
to avoid machine noise). A result showing this design is slower than the C++ it replaces does not necessarily
invalidate the storage strategy (the lock-acquisition-once pattern above may need tuning, or the
benchmark's kernel may not be representative), but it must be resolved — either by revising this
ADR or by explicitly accepting a documented performance cost — before the pattern is propagated
into `vtkPoints` and beyond. This benchmark is sequenced before the separate, broader evidence-audit
of other unproven ROADMAP assumptions (wasm-actually-works-in-a-browser, `wgpu` for volume
rendering — tracked as issue #56): if the port's core numeric layer shows no benefit over C++, that
changes the calculus for investing further in those too.

## Consequences

- Every signature touching a runtime-typed numeric array names `DataArray` — no generic parameter,
  no `Box<dyn Trait>`, no lifetime parameter leaks into caller code.
- Aliasing/identity semantics match VTK's today, so ported tests that rely on two handles sharing
  one array's mutations should port without behavioural surprises from the storage layer itself.
- Every read or write of array contents pays one `RwLock` acquisition per call site that touches
  it, not per element — acceptable only as long as call sites follow the "acquire once, operate on
  the guard's slice" pattern this ADR mandates. A call site that acquires the lock inside a loop is
  a defect against this ADR, not a style nit.
- Ten enum variants (not thirteen) mean `vtkIdTypeArray`, `vtkLongArray`, and `vtkCharArray`-family
  ledger rows map onto shared `I64`/`I32`/`I8` variants rather than getting one each — expected and
  intentional, not a gap.
- `vtkBitArray`, `vtkSOADataArrayTemplate`, and non-numeric `vtkAbstractArray` subclasses have no
  representation yet. Each needs its own follow-up decision when a module actually requires it;
  none currently block Phase 1's entry point.
- The design is provisional until the benchmark task (part of the `vtk-common-core` implementation
  plan) runs. A regression found there is expected to trigger a revision of this document, not a
  silent workaround at the call site.

## Alternatives rejected

- **Generic `TypedArray<T>` with a shared trait, type-erased via `Box<dyn Trait>` at collection
  boundaries.** Doesn't remove the runtime-dispatch enum/vtable, just relocates it to wherever
  heterogeneous arrays are collected, while adding a second representation (concrete `TypedArray<T>`
  vs. the type-erased form) that call sites must know when to use.
- **`Arc<[T]>` with copy-on-write on mutation.** Considered first; rejected because it silently
  detaches a clone from its source on first write, diverging from VTK's actual shared-mutable
  object-identity semantics — a behavioural mismatch that could surface as a subtle, hard-to-trace
  test failure when porting code that depends on aliasing.
- **`Rc<RefCell<Vec<T>>>` (single-threaded interior mutability).** Cheaper per access and simpler
  (panics on conflicting borrows instead of blocking), but migrating a single-threaded design to a
  thread-safe one later touches every call site; paying the small `RwLock` cost up front avoids a
  second migration if/when VTK's multi-threading model gets its own ROADMAP entry.
- **Raw pointer + manual lifetime management, mirroring C++'s `void*` directly.** Rejected outright:
  reintroduces the exact class of memory-safety risk (dangling/aliased raw pointers) that motivates
  doing this port in Rust in the first place.
- **Configurable `vtkIdType` width (32/64-bit via a Cargo feature), mirroring `VTK_USE_64BIT_IDS`.**
  Rejected for v0: doubles the type matrix that needs testing for a configuration axis nothing in
  this project currently exercises. Revisit if a concrete need for 32-bit IDs appears.
