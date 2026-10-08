use std::{fs, path::Path};

fn root() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
}

#[test]
fn documented_error_range_matches_the_idl() {
    let idl: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(root().join("idl/organization.json")).unwrap())
            .unwrap();
    let codes: Vec<u64> = idl["errors"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["code"].as_u64().unwrap())
        .collect();
    let (min, max) = (codes.iter().min().unwrap(), codes.iter().max().unwrap());
    let doc = fs::read_to_string(root().join("docs/organization.md")).unwrap();
    assert!(
        doc.contains(&format!("codes {min}-{max})")),
        "docs/organization.md must say codes {min}-{max}"
    );
    let readme = fs::read_to_string(root().join("README.md")).unwrap();
    assert!(
        readme.contains(&format!("`{min}`-`{max}`")),
        "README.md must say `{min}`-`{max}`"
    );
}
