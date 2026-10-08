use std::{
    fs,
    path::{Path, PathBuf},
    time::SystemTime,
};

fn newest_mtime(path: &Path) -> Option<SystemTime> {
    let meta = fs::metadata(path).ok()?;
    let own = meta.modified().ok();
    if !meta.is_dir() {
        return own;
    }
    fs::read_dir(path)
        .ok()?
        .flatten()
        .filter_map(|e| newest_mtime(&e.path()))
        .chain(own)
        .max()
}

fn is_stale(binary: SystemTime, newest_source: Option<SystemTime>) -> bool {
    newest_source.is_some_and(|s| s > binary)
}

fn source_paths(workspace: &Path, name: &str) -> Vec<PathBuf> {
    let mut crates = vec![name];
    if name == "film" {
        crates.push("organization");
    }
    let mut paths = vec![workspace.join("Cargo.toml"), workspace.join("Cargo.lock")];
    for c in crates {
        let dir = workspace.join("programs").join(c);
        paths.push(dir.join("src"));
        paths.push(dir.join("Cargo.toml"));
    }
    paths
}

fn newest_source(workspace: &Path, name: &str) -> Option<SystemTime> {
    source_paths(workspace, name)
        .iter()
        .filter_map(|p| newest_mtime(p))
        .max()
}

pub fn program_bytes(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("../deploy/{name}.so"));
    let bytes = fs::read(&path).unwrap_or_else(|e| {
        panic!(
            "cannot read {}: {e}; run `anchor build` first",
            path.display()
        )
    });
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let built = fs::metadata(&path).and_then(|m| m.modified()).unwrap();
    let newest = newest_source(&workspace, name);
    if is_stale(built, newest) {
        panic!("target/deploy/{name}.so is older than its sources; run `anchor build`");
    }
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn scratch(tag: &str) -> PathBuf {
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("staleness-{tag}"));
        let _ = fs::remove_dir_all(&dir);
        for c in ["film", "organization"] {
            let prog = dir.join("programs").join(c);
            fs::create_dir_all(prog.join("src")).unwrap();
            fs::write(prog.join("src/lib.rs"), "").unwrap();
            fs::write(prog.join("Cargo.toml"), "").unwrap();
        }
        fs::write(dir.join("Cargo.toml"), "").unwrap();
        fs::write(dir.join("Cargo.lock"), "").unwrap();
        dir
    }

    fn set_mtime(path: &Path, at: SystemTime) {
        fs::File::open(path).unwrap().set_modified(at).unwrap();
    }

    fn detects(tag: &str, name: &str, file: &str) -> bool {
        let dir = scratch(tag);
        let old = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000);
        let binary = old + Duration::from_secs(10);
        for c in ["film", "organization"] {
            let prog = dir.join("programs").join(c);
            for f in [
                prog.join("src"),
                prog.join("src/lib.rs"),
                prog.join("Cargo.toml"),
            ] {
                set_mtime(&f, old);
            }
        }
        set_mtime(&dir.join("Cargo.toml"), old);
        set_mtime(&dir.join("Cargo.lock"), old);
        assert!(!is_stale(binary, newest_source(&dir, name)));
        set_mtime(&dir.join(file), binary + Duration::from_secs(5));
        is_stale(binary, newest_source(&dir, name))
    }

    #[test]
    fn organization_source_makes_film_stale() {
        assert!(detects(
            "org-film",
            "film",
            "programs/organization/src/lib.rs"
        ));
        assert!(!detects(
            "org-org",
            "organization",
            "programs/film/src/lib.rs"
        ));
    }

    #[test]
    fn workspace_manifests_make_programs_stale() {
        for f in ["Cargo.toml", "Cargo.lock"] {
            assert!(detects(&format!("ws-{f}"), "organization", f));
            assert!(detects(&format!("wsf-{f}"), "film", f));
        }
    }

    #[test]
    fn staleness_comparison() {
        let t = SystemTime::UNIX_EPOCH + Duration::from_secs(100);
        let later = t + Duration::from_secs(1);
        assert!(is_stale(t, Some(later)));
        assert!(!is_stale(t, Some(t)));
        assert!(!is_stale(later, Some(t)));
        assert!(!is_stale(t, None));
    }
}
