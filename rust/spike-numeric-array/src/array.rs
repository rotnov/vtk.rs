//! `DataArray`: a runtime-typed numeric buffer, one variant per fixed-width numeric type.
//!
//! Validation spike for docs/decisions/0004-numeric-array-storage.md — not part of the
//! `vtk-common-core` port. See `rust/spike-numeric-array/README.md`.

use std::sync::{Arc, RwLock};

/// One concrete type for "an array of numeric type unknown until runtime" — see ADR 0004.
#[derive(Clone)]
pub enum DataArray {
    F32(Arc<RwLock<Vec<f32>>>),
    F64(Arc<RwLock<Vec<f64>>>),
    I8(Arc<RwLock<Vec<i8>>>),
    U8(Arc<RwLock<Vec<u8>>>),
    I16(Arc<RwLock<Vec<i16>>>),
    U16(Arc<RwLock<Vec<u16>>>),
    I32(Arc<RwLock<Vec<i32>>>),
    U32(Arc<RwLock<Vec<u32>>>),
    I64(Arc<RwLock<Vec<i64>>>),
    U64(Arc<RwLock<Vec<u64>>>),
}

impl DataArray {
    pub fn from_f32(values: Vec<f32>) -> Self {
        DataArray::F32(Arc::new(RwLock::new(values)))
    }
    pub fn from_f64(values: Vec<f64>) -> Self {
        DataArray::F64(Arc::new(RwLock::new(values)))
    }
    pub fn from_i8(values: Vec<i8>) -> Self {
        DataArray::I8(Arc::new(RwLock::new(values)))
    }
    pub fn from_u8(values: Vec<u8>) -> Self {
        DataArray::U8(Arc::new(RwLock::new(values)))
    }
    pub fn from_i16(values: Vec<i16>) -> Self {
        DataArray::I16(Arc::new(RwLock::new(values)))
    }
    pub fn from_u16(values: Vec<u16>) -> Self {
        DataArray::U16(Arc::new(RwLock::new(values)))
    }
    pub fn from_i32(values: Vec<i32>) -> Self {
        DataArray::I32(Arc::new(RwLock::new(values)))
    }
    pub fn from_u32(values: Vec<u32>) -> Self {
        DataArray::U32(Arc::new(RwLock::new(values)))
    }
    pub fn from_i64(values: Vec<i64>) -> Self {
        DataArray::I64(Arc::new(RwLock::new(values)))
    }
    pub fn from_u64(values: Vec<u64>) -> Self {
        DataArray::U64(Arc::new(RwLock::new(values)))
    }

    /// Total element count across the flat buffer (not tuple count). Dispatched once per call —
    /// see ADR 0004's "match once per call, not once per element".
    pub fn len(&self) -> usize {
        match self {
            DataArray::F32(b) => b.read().unwrap().len(),
            DataArray::F64(b) => b.read().unwrap().len(),
            DataArray::I8(b) => b.read().unwrap().len(),
            DataArray::U8(b) => b.read().unwrap().len(),
            DataArray::I16(b) => b.read().unwrap().len(),
            DataArray::U16(b) => b.read().unwrap().len(),
            DataArray::I32(b) => b.read().unwrap().len(),
            DataArray::U32(b) => b.read().unwrap().len(),
            DataArray::I64(b) => b.read().unwrap().len(),
            DataArray::U64(b) => b.read().unwrap().len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_variant_reports_its_length() {
        let cases: Vec<(DataArray, usize)> = vec![
            (DataArray::from_f32(vec![1.0, 2.0, 3.0]), 3),
            (DataArray::from_f64(vec![1.0, 2.0]), 2),
            (DataArray::from_i8(vec![1]), 1),
            (DataArray::from_u8(vec![1, 2, 3, 4]), 4),
            (DataArray::from_i16(vec![]), 0),
            (DataArray::from_u16(vec![1, 2]), 2),
            (DataArray::from_i32(vec![1, 2, 3]), 3),
            (DataArray::from_u32(vec![1]), 1),
            (DataArray::from_i64(vec![1, 2, 3, 4, 5]), 5),
            (DataArray::from_u64(vec![1, 2]), 2),
        ];
        for (array, expected_len) in cases {
            assert_eq!(array.len(), expected_len);
            assert_eq!(array.is_empty(), expected_len == 0);
        }
    }

    #[test]
    fn clone_shares_identity_and_mutations() {
        let original = DataArray::from_f64(vec![1.0, 2.0, 3.0]);
        let alias = original.clone();

        if let DataArray::F64(buf) = &original {
            buf.write().unwrap()[0] = 99.0;
        } else {
            panic!("expected F64 variant");
        }

        if let DataArray::F64(buf) = &alias {
            assert_eq!(buf.read().unwrap()[0], 99.0);
        } else {
            panic!("expected F64 variant");
        }
    }
}
