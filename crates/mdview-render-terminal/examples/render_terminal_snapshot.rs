use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use mdview_render_terminal::{render_str, Registry, RenderCtx};

fn main() -> ExitCode {
    let manifest = env!("CARGO_MANIFEST_DIR");
    let fixtures_dir = Path::new(manifest).join("..").join("..").join("fixtures");
    let snapshot_dir = Path::new(manifest).join("__snapshots__");
    if !snapshot_dir.exists() {
        fs::create_dir_all(&snapshot_dir).expect("create snapshot dir");
    }

    if !fixtures_dir.exists() {
        eprintln!(
            "no fixtures at {}: nothing to snapshot",
            fixtures_dir.display()
        );
        return ExitCode::SUCCESS;
    }

    let ctx = RenderCtx::default();
    let registry = Registry::new();

    let mut paths: Vec<PathBuf> = fs::read_dir(&fixtures_dir)
        .expect("read fixtures")
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("md"))
        .collect();
    paths.sort();

    let mut failures: Vec<String> = Vec::new();
    for fixture in &paths {
        let name = fixture
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");
        let src = fs::read_to_string(fixture).expect("read fixture");
        let ansi = render_str(&src, &ctx, &registry);

        let actual_path = snapshot_dir.join(format!("{}.actual.ansi", name));
        let expected_path = snapshot_dir.join(format!("{}.expected.ansi", name));
        fs::write(&actual_path, &ansi).expect("write actual");

        if expected_path.exists() {
            let expected = fs::read_to_string(&expected_path).expect("read expected");
            if expected != ansi {
                failures.push(name.to_string());
            }
        } else {
            fs::write(&expected_path, &ansi).expect("seed expected");
            eprintln!("seeded expected snapshot for {}", name);
        }
    }

    if failures.is_empty() {
        println!("snapshots ok ({} fixtures)", paths.len());
        ExitCode::SUCCESS
    } else {
        eprintln!("snapshot mismatch: {:?}", failures);
        ExitCode::from(1)
    }
}
