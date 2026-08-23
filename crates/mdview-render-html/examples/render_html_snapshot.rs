use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use mdview_render_html::{render_markdown, Registry, RenderCtx};

fn main() -> ExitCode {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixtures_dir = resolve_fixtures(&crate_dir);
    let snapshots_dir = crate_dir.join("__snapshots__");
    if !snapshots_dir.exists() {
        fs::create_dir_all(&snapshots_dir).expect("create __snapshots__");
    }

    let mut mismatches: Vec<String> = Vec::new();
    let fixtures = read_fixtures(&fixtures_dir);
    if fixtures.is_empty() {
        eprintln!("no fixtures found under {}", fixtures_dir.display());
        return ExitCode::from(2);
    }

    for fixture in fixtures {
        let name = fixture.file_stem().unwrap().to_string_lossy().to_string();
        let src = fs::read_to_string(&fixture).expect("read fixture");
        let html = render_markdown(&src, &RenderCtx::default(), &Registry::new());

        let actual_path = snapshots_dir.join(format!("{name}.actual.html"));
        let expected_path = snapshots_dir.join(format!("{name}.expected.html"));
        fs::write(&actual_path, &html).expect("write actual snapshot");

        if !expected_path.exists() {
            fs::write(&expected_path, &html).expect("write expected snapshot");
            println!("seeded {}", expected_path.display());
            continue;
        }
        let expected = fs::read_to_string(&expected_path).expect("read expected");
        if expected != html.as_str() {
            mismatches.push(name);
        }
    }

    if mismatches.is_empty() {
        println!("all snapshots match");
        ExitCode::SUCCESS
    } else {
        eprintln!("snapshot mismatch: {:?}", mismatches);
        ExitCode::from(1)
    }
}

fn resolve_fixtures(crate_dir: &Path) -> PathBuf {
    let local = crate_dir.join("fixtures");
    if local.exists() {
        return local;
    }
    crate_dir.join("../../fixtures")
}

fn read_fixtures(dir: &Path) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = fs::read_dir(dir)
        .map(|rd| {
            rd.filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| p.extension().map(|e| e == "md").unwrap_or(false))
                .collect()
        })
        .unwrap_or_default();
    out.sort();
    out
}
