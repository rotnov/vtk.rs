mod assertions;
mod cmake_parser;
mod crates;
mod ledger;
mod reference_tree;

use std::path::Path;
use std::process::ExitCode;

use assertions::Violation;
use cmake_parser::ParsedCMakeFile;
use ledger::LedgerRow;

// Mirrors rust/Cargo.toml's [workspace.members] — add new crates here when they're added there.
// (Same manual-list convention as cargo-check-wasm32 in .github/workflows/rust-checks.yml; see
// AGENTS.md § Required checks.)
const WORKSPACE_CRATES: &[&str] = &[
    "vtk-common-core",
    "vtk-common-math",
    "vtk-common-system",
    "vtk-common-transforms",
    "vtk-common-misc",
    "vtk-common-data-model",
    "vtk-common-execution-model",
];

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("ledger-check") => run_ledger_check(),
        Some(other) => {
            eprintln!("unknown xtask command: {other}");
            eprintln!("usage: cargo xtask ledger-check");
            ExitCode::from(2)
        }
        None => {
            eprintln!("usage: cargo xtask ledger-check");
            ExitCode::from(2)
        }
    }
}

fn run_ledger_check() -> ExitCode {
    let repo_root = match reference_tree::repo_root() {
        Ok(root) => root,
        Err(e) => {
            eprintln!("could not determine repo root via git: {e}");
            return ExitCode::FAILURE;
        }
    };

    let ledger_path = repo_root.join("docs/test-mapping.csv");
    let csv_text = match std::fs::read_to_string(&ledger_path) {
        Ok(text) => text,
        Err(e) => {
            eprintln!("could not read {}: {e}", ledger_path.display());
            return ExitCode::FAILURE;
        }
    };
    let rows = match ledger::parse_ledger(&csv_text) {
        Ok(rows) => rows,
        Err(e) => {
            eprintln!("docs/test-mapping.csv: {e}");
            return ExitCode::FAILURE;
        }
    };

    let mut violations = Vec::new();

    violations.extend(assertions::check_exists(&rows, |p| {
        reference_tree::file_exists_in_tree(&repo_root, p)
    }));
    violations.extend(assertions::check_fresh(&rows, |p| {
        reference_tree::blob_sha_at_head(&repo_root, p)
    }));
    violations.extend(assertions::check_complete(
        &rows,
        &parsed_files_for_started_modules(&repo_root, &rows),
    ));
    violations.extend(assertions::check_parity(
        &rows,
        &crate_code_flags(&repo_root),
    ));

    report(&rows, &violations)
}

fn started_modules(rows: &[LedgerRow]) -> Vec<String> {
    let mut modules: Vec<String> = rows
        .iter()
        .filter_map(|r| ledger::module_of(&r.original_path))
        .map(str::to_string)
        .collect();
    modules.sort();
    modules.dedup();
    modules
}

fn parsed_files_for_started_modules(
    repo_root: &Path,
    rows: &[LedgerRow],
) -> Vec<(String, String, ParsedCMakeFile)> {
    let mut out = Vec::new();
    for module in started_modules(rows) {
        for (subdir, macro_name, ext, options) in [
            ("Cxx", "vtk_add_test_cxx", "cxx", cmake_parser::CXX_OPTIONS),
            (
                "Python",
                "vtk_add_test_python",
                "py",
                cmake_parser::PYTHON_OPTIONS,
            ),
        ] {
            let rel_path = format!("{module}/Testing/{subdir}/CMakeLists.txt");
            let full_path = repo_root.join(&rel_path);
            if !full_path.is_file() {
                continue;
            }
            let text = match std::fs::read_to_string(&full_path) {
                Ok(text) => text,
                Err(e) => {
                    eprintln!("warning: could not read {rel_path}: {e}");
                    continue;
                }
            };
            let parsed = cmake_parser::parse_test_macro_calls(&text, macro_name, ext, options);
            out.push((module.clone(), rel_path, parsed));
        }
    }
    out
}

fn crate_code_flags(repo_root: &Path) -> Vec<(String, bool)> {
    WORKSPACE_CRATES
        .iter()
        .map(|name| {
            let src_dir = repo_root.join("rust/crates").join(name).join("src");
            let has_code = crates::crate_has_code_at(&src_dir).unwrap_or(false);
            (name.to_string(), has_code)
        })
        .collect()
}

fn report(rows: &[LedgerRow], violations: &[Violation]) -> ExitCode {
    if violations.is_empty() {
        if rows.is_empty() {
            println!("ledger-check: 0 rows in docs/test-mapping.csv — nothing to check yet");
        } else {
            println!(
                "ledger-check: {} row(s) across {} module(s), no violations",
                rows.len(),
                started_modules(rows).len()
            );
        }
        return ExitCode::SUCCESS;
    }
    for v in violations {
        println!("[{}] {}", v.assertion, v.message);
    }
    println!("ledger-check: {} violation(s)", violations.len());
    ExitCode::FAILURE
}
