use std::path::PathBuf;
use std::process::Command;

fn main() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let webview_dir = manifest_dir.join("webview");
    let dist_dir = webview_dir.join("dist");

    println!(
        "cargo:rerun-if-changed={}",
        webview_dir.join("src").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        webview_dir.join("package.json").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        webview_dir.join("build.mjs").display()
    );
    println!("cargo:rerun-if-env-changed=MDVIEW_SKIP_WEBVIEW_BUILD");

    if std::env::var_os("MDVIEW_SKIP_WEBVIEW_BUILD").is_some() {
        ensure_placeholder(&dist_dir);
        return;
    }

    if !webview_dir.join("package.json").exists() {
        ensure_placeholder(&dist_dir);
        return;
    }

    let runner = if which("bun") {
        Some(("bun", vec!["run", "build"]))
    } else if which("npm") {
        Some(("npm", vec!["run", "build"]))
    } else {
        None
    };

    if let Some((cmd, args)) = runner {
        let status = Command::new(cmd)
            .args(&args)
            .current_dir(&webview_dir)
            .status();
        match status {
            Ok(s) if s.success() => {}
            _ => {
                println!("cargo:warning=webview build failed; using placeholder bundle");
                ensure_placeholder(&dist_dir);
            }
        }
    } else {
        println!("cargo:warning=neither bun nor npm found; using placeholder bundle");
        ensure_placeholder(&dist_dir);
    }
}

fn which(bin: &str) -> bool {
    let names: &[&str] = if cfg!(windows) {
        &[".cmd", ".exe", ".bat", ""]
    } else {
        &[""]
    };
    let path = match std::env::var_os("PATH") {
        Some(p) => p,
        None => return false,
    };
    for dir in std::env::split_paths(&path) {
        for ext in names {
            let candidate = dir.join(format!("{bin}{ext}"));
            if candidate.is_file() {
                return true;
            }
        }
    }
    false
}

fn ensure_placeholder(dist: &PathBuf) {
    let _ = std::fs::create_dir_all(dist);
    let index = dist.join("index.html");
    if !index.exists() {
        let html = "<!doctype html><meta charset=utf-8><title>mdview</title><body><main id=mdv></main></body>";
        let _ = std::fs::write(&index, html);
    }
    let js = dist.join("client.js");
    if !js.exists() {
        let _ = std::fs::write(&js, "// placeholder bundle\n");
    }
}
