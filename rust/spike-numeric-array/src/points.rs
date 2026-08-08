//! `Points`: minimal `vtkPoints`-equivalent wrapper over an `F64` `DataArray`, used only to
//! exercise ADR 0004's storage/dispatch design end-to-end for the validation benchmark.
//!
//! Scope note: real `vtkPoints` accepts any of `vtkDataArray`'s concrete types via
//! `SetDataType`; this spike fixes it to `F64` because the benchmark measures a
//! double-precision bounds kernel against equivalent C++ `double` data. Supporting the other
//! nine `DataArray` variants is deferred to the real `vtkPoints` port task (issue #45).

use crate::array::DataArray;

#[derive(Debug, PartialEq)]
pub enum PointsError {
    /// `DataArray` length is not a multiple of 3 (points are 3-component tuples).
    NotDivisibleByThree,
    /// `Points` requires an `F64`-backed `DataArray` — see the module scope note.
    RequiresF64,
}

pub struct Points {
    data: DataArray,
}

impl Points {
    pub fn new(data: DataArray) -> Result<Self, PointsError> {
        match &data {
            DataArray::F64(buf) => {
                if buf.read().unwrap().len() % 3 != 0 {
                    return Err(PointsError::NotDivisibleByThree);
                }
            }
            _ => return Err(PointsError::RequiresF64),
        }
        Ok(Points { data })
    }

    /// `[xmin, xmax, ymin, ymax, zmin, zmax]`, matching `vtkPoints::GetBounds()`'s layout.
    /// `None` for an empty point set (real `vtkPoints::GetBounds()` on 0 points leaves
    /// `VTK_DOUBLE_MAX`/`-VTK_DOUBLE_MAX` sentinels instead — surfaced here as `None`).
    ///
    /// Dispatches on the `DataArray` variant exactly once per call, then acquires the lock
    /// exactly once — see ADR 0004's "match once per call, not once per element". Only the
    /// `F64` arm is reachable in practice, since `new()` rejects non-`F64` data, but the
    /// `match` must be textually present and executed on every call, since that dispatch is
    /// the literal thing this benchmark exists to measure.
    pub fn bounds(&self) -> Option<[f64; 6]> {
        match &self.data {
            DataArray::F64(buf) => {
                let guard = buf.read().unwrap();
                let mut chunks = guard.chunks_exact(3);
                let first = chunks.next()?;
                let mut bounds = [first[0], first[0], first[1], first[1], first[2], first[2]];
                for p in chunks {
                    if p[0] < bounds[0] {
                        bounds[0] = p[0];
                    }
                    if p[0] > bounds[1] {
                        bounds[1] = p[0];
                    }
                    if p[1] < bounds[2] {
                        bounds[2] = p[1];
                    }
                    if p[1] > bounds[3] {
                        bounds[3] = p[1];
                    }
                    if p[2] < bounds[4] {
                        bounds[4] = p[2];
                    }
                    if p[2] > bounds[5] {
                        bounds[5] = p[2];
                    }
                }
                Some(bounds)
            }
            _ => unreachable!("Points::new rejects non-F64 data"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_non_f64_data_arrays() {
        let data = DataArray::from_i32(vec![1, 2, 3]);
        match Points::new(data) {
            Err(e) => assert_eq!(e, PointsError::RequiresF64),
            Ok(_) => panic!("expected Err(PointsError::RequiresF64)"),
        }
    }

    #[test]
    fn rejects_lengths_not_divisible_by_three() {
        let data = DataArray::from_f64(vec![1.0, 2.0]);
        match Points::new(data) {
            Err(e) => assert_eq!(e, PointsError::NotDivisibleByThree),
            Ok(_) => panic!("expected Err(PointsError::NotDivisibleByThree)"),
        }
    }

    #[test]
    fn bounds_of_empty_points_is_none() {
        let points = Points::new(DataArray::from_f64(vec![])).unwrap();
        assert_eq!(points.bounds(), None);
    }

    #[test]
    fn bounds_of_single_point_is_degenerate() {
        let points = Points::new(DataArray::from_f64(vec![1.0, 2.0, 3.0])).unwrap();
        assert_eq!(points.bounds(), Some([1.0, 1.0, 2.0, 2.0, 3.0, 3.0]));
    }

    #[test]
    fn bounds_of_several_points() {
        let points = Points::new(DataArray::from_f64(vec![
            0.0, 0.0, 0.0, -1.0, 5.0, 2.0, 3.0, -4.0, 2.0,
        ]))
        .unwrap();
        assert_eq!(points.bounds(), Some([-1.0, 3.0, -4.0, 5.0, 0.0, 2.0]));
    }
}
