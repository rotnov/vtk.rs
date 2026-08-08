use std::path::Path;

pub fn crate_has_code<I: IntoIterator<Item = String>>(file_contents: I) -> bool {
    file_contents.into_iter().any(|contents| {
        contents
            .lines()
            .any(|line| !line.trim().is_empty() && !line.trim().starts_with("//"))
    })
}

#[allow(dead_code)] // wired up by Task 6
pub fn crate_has_code_at(src_dir: &Path) -> std::io::Result<bool> {
    let mut file_contents = Vec::new();
    collect_rs_files(src_dir, &mut file_contents)?;
    Ok(crate_has_code(file_contents))
}

fn collect_rs_files(dir: &Path, out: &mut Vec<String>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_rs_files(&path, out)?;
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(std::fs::read_to_string(&path)?);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_single_doc_comment_line_is_not_code() {
        let files = vec!["//! Port of VTK::CommonCore. See ROADMAP.md.\n".to_string()];
        assert!(!crate_has_code(files));
    }

    #[test]
    fn blank_lines_are_not_code() {
        let files = vec!["\n\n  \n".to_string()];
        assert!(!crate_has_code(files));
    }

    #[test]
    fn a_real_declaration_is_code() {
        let files = vec!["//! doc\npub struct Foo;\n".to_string()];
        assert!(crate_has_code(files));
    }

    #[test]
    fn code_in_a_second_file_still_counts() {
        let files = vec![
            "//! doc only\n".to_string(),
            "pub fn helper() {}\n".to_string(),
        ];
        assert!(crate_has_code(files));
    }

    #[test]
    fn no_files_is_not_code() {
        let files: Vec<String> = vec![];
        assert!(!crate_has_code(files));
    }
}
