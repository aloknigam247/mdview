use std::path::Path;

use walkdir::WalkDir;

#[test]
fn source_tree_has_no_pending_markers() {
    let todo = ["TO", "DO"].concat();
    let fixme = ["FIX", "ME"].concat();
    let exts = ["rs", "ts", "js", "lua"];
    let skip_dirs = [".git", "assets", "node_modules", "target", "vendor"];
    let this_file = Path::new(file!()).file_name().unwrap().to_owned();

    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("workspace root")
        .to_path_buf();

    let mut hits = Vec::new();
    for entry in WalkDir::new(&root).into_iter().filter_entry(|entry| {
        !entry
            .file_name()
            .to_str()
            .map(|name| skip_dirs.contains(&name))
            .unwrap_or(false)
    }) {
        let entry = entry.expect("walk");
        let path = entry.path();
        if path.file_name() == Some(this_file.as_os_str()) {
            continue;
        }
        let is_src = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| exts.contains(&ext))
            .unwrap_or(false);
        if !is_src {
            continue;
        }
        if let Ok(text) = std::fs::read_to_string(path) {
            for (line_number, line) in text.lines().enumerate() {
                if line.contains(&todo) || line.contains(&fixme) {
                    hits.push(format!(
                        "{}:{}: {}",
                        path.display(),
                        line_number + 1,
                        line.trim()
                    ));
                }
            }
        }
    }

    assert!(
        hits.is_empty(),
        "unexpected pending markers left in source:\n{}",
        hits.join("\n")
    );
}
