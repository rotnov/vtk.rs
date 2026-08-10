//! Validation spike for docs/decisions/0004-numeric-array-storage.md.
//!
//! Not part of the `vtk-common-core` port — see `README.md` in this crate's root. Exists only
//! to run the `DataArray`/`Points` design through a real dispatch-and-bounds workload and
//! benchmark it against equivalent C++, per the ADR's "Validation is required" clause.

pub mod array;
pub mod points;

pub use array::DataArray;
pub use points::{Points, PointsError};
