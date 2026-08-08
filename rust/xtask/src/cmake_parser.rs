#[allow(dead_code)] // Wired up by Task 6 in orchestration
pub const CXX_OPTIONS: &[&str] = &[
    "NO_DATA",
    "NO_VALID",
    "NO_OUTPUT",
    "TIGHT_VALID",
    "LOOSE_VALID",
    "LEGACY_VALID",
    "WEBGPU_GRAPHICS_BACKEND",
];

#[allow(dead_code)] // Wired up by Task 6 in orchestration
pub const PYTHON_OPTIONS: &[&str] = &[
    "NO_DATA",
    "NO_VALID",
    "NO_OUTPUT",
    "NO_RT",
    "DIRECT_DATA",
    "JUST_VALID",
    "LEGACY_VALID",
    "TIGHT_VALID",
    "LOOSE_VALID",
];

#[allow(dead_code)] // Wired up by Task 6 in orchestration
#[derive(Debug, PartialEq)]
pub struct ParsedTest {
    pub name: String,
    pub raw_token: String,
}

#[allow(dead_code)] // Wired up by Task 6 in orchestration
#[derive(Debug, Default)]
pub struct ParsedCMakeFile {
    pub tests: Vec<ParsedTest>,
    pub unresolved: Vec<String>,
}

#[allow(dead_code)] // Wired up by Task 6 in orchestration
pub fn parse_test_macro_calls(
    text: &str,
    macro_name: &str,
    ext: &str,
    known_options: &[&str],
) -> ParsedCMakeFile {
    let mut result = ParsedCMakeFile::default();
    let stripped = strip_comments(text);
    for call_args in find_macro_call_args(&stripped, macro_name) {
        let tokens = tokenize_args(&call_args);
        // Skip the first two positional args: EXENAME and VARNAME.
        for token in tokens.into_iter().skip(2) {
            classify_token(&token, ext, known_options, &mut result);
        }
    }
    result
}

#[allow(dead_code)] // Wired up by Task 6 in orchestration
fn strip_comments(text: &str) -> String {
    text.lines()
        .map(|line| match line.find('#') {
            Some(idx) => &line[..idx],
            None => line,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Finds every `macro_name(...)` call in `text` and returns the raw text between the matching
/// parentheses for each one. Assumes no nested parentheses inside test macro calls (true for
/// every vtk_add_test_cxx/vtk_add_test_python call in the reference tree).
#[allow(dead_code)] // Wired up by Task 6 in orchestration
fn find_macro_call_args(text: &str, macro_name: &str) -> Vec<String> {
    let mut calls = Vec::new();
    let mut search_from = 0;
    while let Some(rel_start) = text[search_from..].find(macro_name) {
        let start = search_from + rel_start;
        let after_name = start + macro_name.len();
        // Require the next non-whitespace char to be '(' so we don't match a longer identifier
        // that happens to contain macro_name as a substring.
        let open_paren = match text[after_name..].find(|c: char| !c.is_whitespace()) {
            Some(off) if text[after_name + off..].starts_with('(') => after_name + off,
            _ => {
                search_from = after_name;
                continue;
            }
        };
        let close_paren = match text[open_paren..].find(')') {
            Some(off) => open_paren + off,
            None => break,
        };
        calls.push(text[open_paren + 1..close_paren].to_string());
        search_from = close_paren + 1;
    }
    calls
}

#[allow(dead_code)] // Wired up by Task 6 in orchestration
fn tokenize_args(args: &str) -> Vec<String> {
    args.split_whitespace().map(|s| s.to_string()).collect()
}

#[allow(dead_code)] // Wired up by Task 6 in orchestration
fn classify_token(token: &str, ext: &str, known_options: &[&str], result: &mut ParsedCMakeFile) {
    if known_options.contains(&token) {
        return;
    }
    if let Some(var_name) = token.strip_prefix("${").and_then(|s| s.strip_suffix('}')) {
        result.unresolved.push(var_name.to_string());
        return;
    }
    let ext_suffix = format!(".{ext}");
    if let Some((left, right)) = token.split_once(',') {
        let name = if left.ends_with(&ext_suffix) {
            left.trim_end_matches(&ext_suffix).to_string()
        } else {
            left.to_string()
        };
        result.tests.push(ParsedTest {
            name,
            raw_token: token.to_string(),
        });
        let _ = right; // kept only for readability; not needed further.
        return;
    }
    if token.ends_with(&ext_suffix) {
        result.tests.push(ParsedTest {
            name: token.trim_end_matches(&ext_suffix).to_string(),
            raw_token: token.to_string(),
        });
    }
    // Anything else (e.g. a stray positional arg) is silently ignored — v1 scope is test
    // extraction, not full argument validation.
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(result: &ParsedCMakeFile) -> Vec<&str> {
        result.tests.iter().map(|t| t.name.as_str()).collect()
    }

    #[test]
    fn bare_form_strips_extension() {
        let text = "vtk_add_test_cxx(vtkCommonCoreCxx tests\n  TestArrayAPI.cxx\n  )";
        let result = parse_test_macro_calls(text, "vtk_add_test_cxx", "cxx", CXX_OPTIONS);
        assert_eq!(names(&result), vec!["TestArrayAPI"]);
        assert!(result.unresolved.is_empty());
    }

    #[test]
    fn per_test_comma_options_form_strips_extension() {
        let text = "vtk_add_test_cxx(vtkCommonCoreCxx tests\n  TestArrayAPI.cxx,NO_VALID\n  )";
        let result = parse_test_macro_calls(text, "vtk_add_test_cxx", "cxx", CXX_OPTIONS);
        assert_eq!(names(&result), vec!["TestArrayAPI"]);
    }

    #[test]
    fn custom_name_and_path_comma_form_keeps_name_unstripped() {
        // Mirrors Common/Core/Testing/Cxx/CMakeLists.txt's add_data_array_test-generated entries.
        let text = "vtk_add_test_cxx(vtkCommonCoreCxx tests\n  \
            TestDataArrayAPI_vtkCharArray,${CMAKE_CURRENT_BINARY_DIR}/TestDataArrayAPI_vtkCharArray.cxx\n  )";
        let result = parse_test_macro_calls(text, "vtk_add_test_cxx", "cxx", CXX_OPTIONS);
        assert_eq!(names(&result), vec!["TestDataArrayAPI_vtkCharArray"]);
    }

    #[test]
    fn block_level_bare_option_is_not_a_test() {
        let text = "vtk_add_test_cxx(vtkCommonCoreCxx tests\n  NO_VALID\n  TestFoo.cxx\n  )";
        let result = parse_test_macro_calls(text, "vtk_add_test_cxx", "cxx", CXX_OPTIONS);
        assert_eq!(names(&result), vec!["TestFoo"]);
    }

    #[test]
    fn unresolved_variable_token_is_flagged_not_silently_dropped() {
        let text = "vtk_add_test_cxx(vtkCommonCoreCxx tests\n  ${data_array_tests}\n  )";
        let result = parse_test_macro_calls(text, "vtk_add_test_cxx", "cxx", CXX_OPTIONS);
        assert!(result.tests.is_empty());
        assert_eq!(result.unresolved, vec!["data_array_tests"]);
    }

    #[test]
    fn mixed_literals_and_variable_in_one_call() {
        // The actual shape of Common/Core/Testing/Cxx/CMakeLists.txt's main vtk_add_test_cxx call:
        // literal test names followed by a spliced-in variable.
        let text = "vtk_add_test_cxx(vtkCommonCoreCxx tests\n  TestArrayAPI.cxx\n  ${data_array_tests}\n  )";
        let result = parse_test_macro_calls(text, "vtk_add_test_cxx", "cxx", CXX_OPTIONS);
        assert_eq!(names(&result), vec!["TestArrayAPI"]);
        assert_eq!(result.unresolved, vec!["data_array_tests"]);
    }

    #[test]
    fn multiple_macro_calls_in_one_file_are_all_parsed() {
        let text = "vtk_add_test_cxx(vtkCommonCoreCxx tests1\n  TestA.cxx\n  )\n\
                     vtk_add_test_cxx(vtkCommonCoreCxx tests2\n  TestB.cxx\n  )";
        let result = parse_test_macro_calls(text, "vtk_add_test_cxx", "cxx", CXX_OPTIONS);
        let mut found = names(&result);
        found.sort();
        assert_eq!(found, vec!["TestA", "TestB"]);
    }

    #[test]
    fn comment_lines_are_ignored() {
        let text =
            "vtk_add_test_cxx(vtkCommonCoreCxx tests\n  # TestIgnored.cxx\n  TestKept.cxx\n  )";
        let result = parse_test_macro_calls(text, "vtk_add_test_cxx", "cxx", CXX_OPTIONS);
        assert_eq!(names(&result), vec!["TestKept"]);
    }

    #[test]
    fn python_macro_uses_python_options_and_py_extension() {
        let text =
            "vtk_add_test_python(vtkCommonCorePython tests\n  DIRECT_DATA\n  TestFoo.py\n  )";
        let result = parse_test_macro_calls(text, "vtk_add_test_python", "py", PYTHON_OPTIONS);
        assert_eq!(names(&result), vec!["TestFoo"]);
    }

    #[test]
    fn a_call_to_a_different_macro_name_is_ignored() {
        let text = "vtk_add_test_mpi(vtkCommonCoreCxx tests\n  TestMpi.cxx\n  )";
        let result = parse_test_macro_calls(text, "vtk_add_test_cxx", "cxx", CXX_OPTIONS);
        assert!(result.tests.is_empty());
    }
}
