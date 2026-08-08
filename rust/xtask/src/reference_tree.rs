use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

#[allow(dead_code)] // wired up by Task 6
pub fn repo_root() -> io::Result<PathBuf> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()?;
    if !output.status.success() {
        return Err(io::Error::other(format!(
            "git rev-parse --show-toplevel failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(PathBuf::from(path))
}

#[allow(dead_code)] // wired up by Task 6
pub fn file_exists_in_tree(repo_root: &Path, rel_path: &str) -> bool {
    repo_root.join(rel_path).is_file()
}

#[allow(dead_code)] // wired up by Task 6
pub fn blob_sha_at_head(repo_root: &Path, rel_path: &str) -> Option<String> {
    let output = Command::new("git")
        .arg("rev-parse")
        .arg(format!("HEAD:{rel_path}"))
        .current_dir(repo_root)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let sha = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if sha.is_empty() { None } else { Some(sha) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repo_root_finds_the_real_worktree_root() {
        let root = repo_root().unwrap();
        assert!(root.join("AGENTS.md").is_file());
    }

    #[test]
    fn file_exists_in_tree_is_true_for_a_real_file() {
        let root = repo_root().unwrap();
        assert!(file_exists_in_tree(&root, "rust/Cargo.toml"));
    }

    #[test]
    fn file_exists_in_tree_is_false_for_a_missing_file() {
        let root = repo_root().unwrap();
        assert!(!file_exists_in_tree(&root, "does/not/exist.cxx"));
    }

    #[test]
    fn blob_sha_at_head_returns_a_40_char_sha_for_a_committed_file() {
        let root = repo_root().unwrap();
        let sha = blob_sha_at_head(&root, "rust/Cargo.toml").unwrap();
        assert_eq!(sha.len(), 40);
        assert!(sha.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn blob_sha_at_head_returns_none_for_a_missing_path() {
        let root = repo_root().unwrap();
        assert!(blob_sha_at_head(&root, "does/not/exist.cxx").is_none());
    }
}
