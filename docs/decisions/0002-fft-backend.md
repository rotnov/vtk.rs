# 0002 — FFT backend for `vtk-common-math`

Status: accepted
Date: 2026-08-06

## Context

`Common/Math/vtk.module` declares `DEPENDS: CommonCore, kissfft`. The kissfft dependency has
exactly one consumer: `vtkFFT`. `Common/Math/vtkFFT.h` and `Common/Math/vtkFFT.cxx` are the only
files in the module that reference it, and the coupling is shallow but total —
`vtkFFT::ScalarNumber` and `vtkFFT::ComplexNumber` are typedefs of `kiss_fft_scalar` and
`kiss_fft_cpx`, so kissfft's types leak into the public API.

Two things about the surface are worth stating up front, because they push in opposite
directions.

The *transform* is a commodity. Nothing about VTK's use of it is unusual: forward and inverse,
complex-to-complex and real-to-complex.

The *layer above the transform* is not, and has to be ported whatever backs it. `vtkFFT.h`
exposes `Fft`/`IFft`/`RFft`/`IRFft` in several overloads, `FftFreq`/`RFftFreq`, window generators
(Hanning, Bartlett, Sine, Blackman, Rectangular) with 1D and 2D kernel generation,
`OverlappingFft`, `Spectrogram`, `Csd` (cross-spectral density), octave-band frequency ranges
with subdivisions, and `Scaling`/`SpectralMode` options. That is a `scipy.signal`-shaped module,
not a thin wrapper, and it is where the porting effort actually is.

## Decision

Back `vtkFFT` with **`rustfft`** for complex transforms and **`realfft`** for the
real-input/real-output paths. Port the layer above them as ordinary Rust code in
`vtk-common-math`.

| crate | version at decision time | license | note |
|---|---|---|---|
| `rustfft` | 6.4.1 | MIT OR Apache-2.0 | pure Rust, runtime-dispatched SIMD (AVX/SSE/NEON) |
| `realfft` | 3.5.0 | MIT | real-to-complex / complex-to-real on top of `rustfft` |

Both licenses are permissive and compatible with the BSD-3-Clause line this port carries.

Do not expose the backend's types in the crate's public API. `vtkFFT` leaks `kiss_fft_cpx`
through its typedefs; the Rust port must not repeat that with `rustfft::num_complex::Complex`
re-exported as its own type, or swapping the backend later becomes a breaking change.

## Consequences

- No C dependency enters `vtk-common-math`, so `Common*` stays a pure-Rust, no-build-script
  layer. This matters more than convenience: a C build dependency in a foundational crate would
  complicate every downstream build and sit awkwardly with the 100% coverage gate, which cannot
  see across an FFI boundary.
- **Results will not be bit-identical to kissfft.** Different algorithms and different
  operation orders round differently. Ported `vtkFFT` tests must compare with a tolerance rather
  than for equality, and any test transcribed from upstream expected values needs its tolerance
  chosen deliberately, not copied. This is the first concrete case for the FFI-oracle idea in
  `ROADMAP.md` § Open questions — comparing against a real VTK build is the only honest way to
  bound the difference.
- Effort is in the signal-processing layer, not the backend. Sizing `vtk-common-math` as "wire up
  an FFT crate" would be wrong by a wide margin; `Spectrogram`, `Csd`, the scaling modes and the
  octave-band helpers are each real work with real tests.
- `realfft` is a second dependency that must track `rustfft`'s major version. Acceptable — it is
  maintained in step with it — but it is a coupling to watch on upgrades.

## Alternatives rejected

- **FFI bindings to `kissfft`.** Would give bit-identical results, which is genuinely tempting
  for a port validated against upstream. Rejected because it puts a C toolchain requirement into
  the foundation of the workspace, leaves code the coverage gate cannot measure, and preserves
  kissfft's types in the public API. The bit-identity argument is better served by an FFI test
  oracle used only in tests, which does not constrain the shipping crate.
- **Hand-port `kissfft` to Rust.** No upside. It would be new, unvalidated code doing what a
  mature, heavily-used crate already does, and it would still need the same tolerance-based
  tests against upstream.
- **A larger numerics framework (`ndarray`-based stacks, FFTW bindings).** FFTW is GPL/commercial
  dual-licensed and a C dependency — both disqualifying. A general numerics framework is far more
  dependency than one module needs.
