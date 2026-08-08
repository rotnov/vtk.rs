use crate::cmake_parser::ParsedCMakeFile;
use crate::ledger::{LedgerRow, module_of};

#[derive(Debug, PartialEq)]
pub struct Violation {
    pub assertion: &'static str,
    pub message: String,
}

#[allow(dead_code)] // Test helper, used only under cfg(test)
fn row(
    original_path: &str,
    original_test: &str,
    original_sha: &str,
    rust_path: &str,
    status: &str,
    notes: &str,
) -> LedgerRow {
    LedgerRow {
        original_path: original_path.to_string(),
        original_test: original_test.to_string(),
        original_sha: original_sha.to_string(),
        rust_path: rust_path.to_string(),
        rust_test: String::new(),
        category: "1".to_string(),
        status: status.to_string(),
        notes: notes.to_string(),
    }
}

pub fn check_exists(rows: &[LedgerRow], exists_fn: impl Fn(&str) -> bool) -> Vec<Violation> {
    rows.iter()
        .filter(|r| !exists_fn(&r.original_path))
        .map(|r| Violation {
            assertion: "exists",
            message: format!(
                "{}: original_path not found in reference tree",
                r.original_path
            ),
        })
        .collect()
}

pub fn check_fresh(rows: &[LedgerRow], sha_fn: impl Fn(&str) -> Option<String>) -> Vec<Violation> {
    rows.iter()
        .filter_map(|r| {
            let current = sha_fn(&r.original_path)?;
            if current != r.original_sha {
                Some(Violation {
                    assertion: "fresh",
                    message: format!(
                        "{} ({}): ledger sha {} but current blob sha {} — re-read and re-port, then update original_sha",
                        r.original_path, r.original_test, r.original_sha, current
                    ),
                })
            } else {
                None
            }
        })
        .collect()
}

/// Extracts the subdirectory segment right after `/Testing/` from a path string,
/// e.g. `Common/Core/Testing/Cxx/CMakeLists.txt` -> `Some("Cxx")`. Used to distinguish
/// the Cxx and Python test registrations within the same module, which can otherwise
/// collide on identically-named tests (e.g. both register a `TestVariant`).
fn testing_subdir(path: &str) -> Option<&str> {
    path.split_once("/Testing/")
        .and_then(|(_, rest)| rest.split('/').next())
}

pub fn check_complete(
    rows: &[LedgerRow],
    parsed_files: &[(String, String, ParsedCMakeFile)],
) -> Vec<Violation> {
    let mut violations = Vec::new();
    for (module, file_path, parsed) in parsed_files {
        let module_rows: Vec<&LedgerRow> = rows
            .iter()
            .filter(|r| module_of(&r.original_path) == Some(module.as_str()))
            .collect();

        for test in &parsed.tests {
            let has_row = module_rows.iter().any(|r| {
                r.original_test == test.name
                    && testing_subdir(&r.original_path) == testing_subdir(file_path)
            });
            if !has_row {
                violations.push(Violation {
                    assertion: "complete",
                    message: format!(
                        "{file_path}: test '{}' is registered but has no row in docs/test-mapping.csv",
                        test.name
                    ),
                });
            }
        }

        for var in &parsed.unresolved {
            let marker = format!("generated:{var}");
            let cleared = module_rows.iter().any(|r| r.notes.contains(&marker));
            if !cleared {
                violations.push(Violation {
                    assertion: "complete",
                    message: format!(
                        "{file_path}: unresolved dynamic test list ${{{var}}} — cannot verify completeness. \
                         Manually enumerate its tests into docs/test-mapping.csv, with at least one row's \
                         notes containing '{marker}' once done."
                    ),
                });
            }
        }
    }
    violations
}

pub fn check_parity(rows: &[LedgerRow], crates: &[(String, bool)]) -> Vec<Violation> {
    crates
        .iter()
        .filter(|(_, has_code)| *has_code)
        .filter_map(|(crate_name, _)| {
            let prefix = format!("rust/crates/{crate_name}/");
            let has_ported_row = rows
                .iter()
                .any(|r| r.status == "ported" && r.rust_path.starts_with(&prefix));
            if has_ported_row {
                None
            } else {
                Some(Violation {
                    assertion: "parity",
                    message: format!(
                        "{crate_name}: contains code but has no docs/test-mapping.csv row with status=ported"
                    ),
                })
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cmake_parser::ParsedTest;

    // ---- exists ----

    #[test]
    fn exists_passes_when_every_path_is_present() {
        let rows = vec![row(
            "Common/Core/Testing/Cxx/TestFoo.cxx",
            "TestFoo",
            "sha",
            "p",
            "ported",
            "",
        )];
        let violations = check_exists(&rows, |_| true);
        assert!(violations.is_empty());
    }

    #[test]
    fn exists_fails_when_a_path_is_missing() {
        let rows = vec![row(
            "Common/Core/Testing/Cxx/TestGone.cxx",
            "TestGone",
            "sha",
            "p",
            "ported",
            "",
        )];
        let violations = check_exists(&rows, |_| false);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].assertion, "exists");
    }

    // ---- fresh ----

    #[test]
    fn fresh_passes_when_sha_matches() {
        let rows = vec![row(
            "Common/Core/Testing/Cxx/TestFoo.cxx",
            "TestFoo",
            "abc",
            "p",
            "ported",
            "",
        )];
        let violations = check_fresh(&rows, |_| Some("abc".to_string()));
        assert!(violations.is_empty());
    }

    #[test]
    fn fresh_fails_when_sha_has_drifted() {
        let rows = vec![row(
            "Common/Core/Testing/Cxx/TestFoo.cxx",
            "TestFoo",
            "abc",
            "p",
            "ported",
            "",
        )];
        let violations = check_fresh(&rows, |_| Some("def".to_string()));
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].assertion, "fresh");
    }

    #[test]
    fn fresh_ignores_a_row_whose_path_has_no_current_sha() {
        // A missing path is check_exists's job to flag; check_fresh must not
        // double-report it by treating a None lookup as a drifted sha.
        let rows = vec![row(
            "Common/Core/Testing/Cxx/TestGone.cxx",
            "TestGone",
            "abc",
            "p",
            "ported",
            "",
        )];
        let violations = check_fresh(&rows, |_| None);
        assert!(violations.is_empty());
    }

    // ---- complete ----

    #[test]
    fn complete_passes_when_every_parsed_test_has_a_row() {
        let rows = vec![row(
            "Common/Core/Testing/Cxx/TestFoo.cxx",
            "TestFoo",
            "sha",
            "p",
            "ported",
            "",
        )];
        let parsed = vec![(
            "Common/Core".to_string(),
            "Common/Core/Testing/Cxx/CMakeLists.txt".to_string(),
            ParsedCMakeFile {
                tests: vec![ParsedTest {
                    name: "TestFoo".to_string(),
                    raw_token: "TestFoo.cxx".to_string(),
                }],
                unresolved: vec![],
            },
        )];
        let violations = check_complete(&rows, &parsed);
        assert!(violations.is_empty());
    }

    #[test]
    fn complete_fails_when_a_registered_test_has_no_row() {
        let rows: Vec<LedgerRow> = vec![row(
            "Common/Core/Testing/Cxx/TestOther.cxx",
            "TestOther",
            "sha",
            "p",
            "ported",
            "",
        )];
        let parsed = vec![(
            "Common/Core".to_string(),
            "Common/Core/Testing/Cxx/CMakeLists.txt".to_string(),
            ParsedCMakeFile {
                tests: vec![ParsedTest {
                    name: "TestMissing".to_string(),
                    raw_token: "TestMissing.cxx".to_string(),
                }],
                unresolved: vec![],
            },
        )];
        let violations = check_complete(&rows, &parsed);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].assertion, "complete");
        assert!(violations[0].message.contains("TestMissing"));
    }

    #[test]
    fn complete_fails_on_an_unresolved_variable_with_no_clearing_note() {
        let rows = vec![row(
            "Common/Core/Testing/Cxx/TestFoo.cxx",
            "TestFoo",
            "sha",
            "p",
            "ported",
            "",
        )];
        let parsed = vec![(
            "Common/Core".to_string(),
            "Common/Core/Testing/Cxx/CMakeLists.txt".to_string(),
            ParsedCMakeFile {
                tests: vec![ParsedTest {
                    name: "TestFoo".to_string(),
                    raw_token: "TestFoo.cxx".to_string(),
                }],
                unresolved: vec!["data_array_tests".to_string()],
            },
        )];
        let violations = check_complete(&rows, &parsed);
        assert_eq!(violations.len(), 1);
        assert!(violations[0].message.contains("data_array_tests"));
        assert!(violations[0].message.contains("generated:data_array_tests"));
    }

    #[test]
    fn complete_clears_an_unresolved_variable_when_a_row_carries_the_marker() {
        let rows = vec![row(
            "Common/Core/Testing/Cxx/TestDataArrayAPI.cxx",
            "TestDataArrayAPI_vtkCharArray",
            "sha",
            "p",
            "ported",
            "generated:data_array_tests",
        )];
        let parsed = vec![(
            "Common/Core".to_string(),
            "Common/Core/Testing/Cxx/CMakeLists.txt".to_string(),
            ParsedCMakeFile {
                tests: vec![],
                unresolved: vec!["data_array_tests".to_string()],
            },
        )];
        let violations = check_complete(&rows, &parsed);
        assert!(violations.is_empty());
    }

    #[test]
    fn complete_does_not_let_a_cxx_row_satisfy_the_same_named_python_test() {
        // Common/Core/Testing/Cxx/CMakeLists.txt and .../Python/CMakeLists.txt both register
        // a test named "TestVariant". A ledger row for the Cxx test must not silently satisfy
        // the Python registration too — they are different files with different tests.
        let rows = vec![row(
            "Common/Core/Testing/Cxx/TestVariant.cxx",
            "TestVariant",
            "sha",
            "p",
            "ported",
            "",
        )];
        let parsed = vec![
            (
                "Common/Core".to_string(),
                "Common/Core/Testing/Cxx/CMakeLists.txt".to_string(),
                ParsedCMakeFile {
                    tests: vec![ParsedTest {
                        name: "TestVariant".to_string(),
                        raw_token: "TestVariant.cxx".to_string(),
                    }],
                    unresolved: vec![],
                },
            ),
            (
                "Common/Core".to_string(),
                "Common/Core/Testing/Python/CMakeLists.txt".to_string(),
                ParsedCMakeFile {
                    tests: vec![ParsedTest {
                        name: "TestVariant".to_string(),
                        raw_token: "TestVariant.py".to_string(),
                    }],
                    unresolved: vec![],
                },
            ),
        ];
        let violations = check_complete(&rows, &parsed);

        // Exactly one violation: the Python TestVariant, which has no row of its own.
        assert_eq!(violations.len(), 1);
        assert!(
            violations[0]
                .message
                .contains("Common/Core/Testing/Python/CMakeLists.txt")
        );
        assert!(violations[0].message.contains("TestVariant"));

        // The Cxx TestVariant must not be reported — it's satisfied by the Cxx row — and both
        // parsed file entries must actually have been evaluated (a subdir-level module key
        // regression would instead make the module lookup for Python come up empty and either
        // over- or under-report here).
        assert!(
            !violations
                .iter()
                .any(|v| v.message.contains("Common/Core/Testing/Cxx/CMakeLists.txt"))
        );
    }

    // ---- parity ----

    #[test]
    fn parity_passes_when_a_ported_row_matches_the_crate() {
        let rows = vec![row(
            "Common/Core/Testing/Cxx/TestFoo.cxx",
            "TestFoo",
            "sha",
            "rust/crates/vtk-common-core/src/foo.rs",
            "ported",
            "",
        )];
        let crates = vec![("vtk-common-core".to_string(), true)];
        let violations = check_parity(&rows, &crates);
        assert!(violations.is_empty());
    }

    #[test]
    fn parity_fails_when_a_crate_has_code_but_no_ported_row() {
        let rows: Vec<LedgerRow> = vec![];
        let crates = vec![("vtk-common-core".to_string(), true)];
        let violations = check_parity(&rows, &crates);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].assertion, "parity");
        assert!(violations[0].message.contains("vtk-common-core"));
    }

    #[test]
    fn parity_ignores_a_crate_with_no_code_yet() {
        let rows: Vec<LedgerRow> = vec![];
        let crates = vec![("vtk-common-core".to_string(), false)];
        let violations = check_parity(&rows, &crates);
        assert!(violations.is_empty());
    }

    #[test]
    fn parity_does_not_count_a_deferred_row_as_ported() {
        let rows = vec![row(
            "Common/Core/Testing/Cxx/TestFoo.cxx",
            "TestFoo",
            "sha",
            "rust/crates/vtk-common-core/src/foo.rs",
            "deferred",
            "phase 2",
        )];
        let crates = vec![("vtk-common-core".to_string(), true)];
        let violations = check_parity(&rows, &crates);
        assert_eq!(violations.len(), 1);
    }
}
