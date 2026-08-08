pub struct LedgerRow {
    pub original_path: String,
    pub original_test: String,
    pub original_sha: String,
    pub rust_path: String,
    // Parsed to keep the CSV schema round-trip complete; no current assertion reads these two
    // columns, so clippy sees them as dead without this allow.
    #[allow(dead_code)]
    pub rust_test: String,
    #[allow(dead_code)]
    pub category: String,
    pub status: String,
    pub notes: String,
}

pub fn parse_ledger(csv_text: &str) -> Result<Vec<LedgerRow>, String> {
    let mut reader = csv::ReaderBuilder::new().from_reader(csv_text.as_bytes());
    let mut rows = Vec::new();
    for (i, result) in reader.records().enumerate() {
        let record = result.map_err(|e| format!("row {}: {e}", i + 1))?;
        if record.len() != 8 {
            return Err(format!(
                "row {}: expected 8 columns, found {}",
                i + 1,
                record.len()
            ));
        }
        rows.push(LedgerRow {
            original_path: record[0].to_string(),
            original_test: record[1].to_string(),
            original_sha: record[2].to_string(),
            rust_path: record[3].to_string(),
            rust_test: record[4].to_string(),
            category: record[5].to_string(),
            status: record[6].to_string(),
            notes: record[7].to_string(),
        });
    }
    Ok(rows)
}

pub fn module_of(original_path: &str) -> Option<&str> {
    original_path
        .split_once("/Testing/")
        .map(|(module, _)| module)
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEADER: &str =
        "original_path,original_test,original_sha,rust_path,rust_test,category,status,notes\n";

    #[test]
    fn empty_ledger_has_no_rows() {
        let rows = parse_ledger(HEADER).unwrap();
        assert!(rows.is_empty());
    }

    #[test]
    fn parses_a_single_row() {
        let csv = format!(
            "{HEADER}Common/Core/Testing/Cxx/TestArrayAPI.cxx,TestArrayAPI,abc123,\
             rust/crates/vtk-common-core/src/array/api.rs,array_api_roundtrip,1,ported,\n"
        );
        let rows = parse_ledger(&csv).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].original_test, "TestArrayAPI");
        assert_eq!(rows[0].status, "ported");
    }

    #[test]
    fn parses_multiple_rows_across_modules() {
        let csv = format!(
            "{HEADER}\
             Common/Core/Testing/Cxx/TestArrayAPI.cxx,TestArrayAPI,sha1,p1,t1,1,ported,\n\
             Common/Math/Testing/Cxx/TestMath.cxx,TestMath,sha2,p2,t2,1,deferred,phase 2\n"
        );
        let rows = parse_ledger(&csv).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[1].notes, "phase 2");
    }

    #[test]
    fn a_note_containing_a_comma_round_trips_when_quoted() {
        let csv = format!(
            "{HEADER}Common/Core/Testing/Cxx/TestFoo.cxx,TestFoo,sha,p,t,1,skipped,\
             \"no Rust analogue, see ADR 0001\"\n"
        );
        let rows = parse_ledger(&csv).unwrap();
        assert_eq!(rows[0].notes, "no Rust analogue, see ADR 0001");
    }

    #[test]
    fn malformed_row_is_an_error_not_a_panic() {
        let csv = format!("{HEADER}too,few,columns\n");
        assert!(parse_ledger(&csv).is_err());
    }

    #[test]
    fn module_of_strips_at_testing_segment() {
        assert_eq!(
            module_of("Common/Core/Testing/Cxx/TestArrayAPI.cxx"),
            Some("Common/Core")
        );
        assert_eq!(
            module_of("Common/DataModel/Testing/Python/TestFoo.py"),
            Some("Common/DataModel")
        );
    }

    #[test]
    fn module_of_returns_none_without_a_testing_segment() {
        assert_eq!(module_of("Common/Core/vtkObject.cxx"), None);
    }
}
