use std::path::PathBuf;
use std::process::Command;

fn main() {
    build_icons();
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

fn build_icons() {
    use std::fs;
    use std::path::PathBuf;
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let svg = manifest.join("assets").join("icon").join("icon.svg");
    println!("cargo:rerun-if-changed={}", svg.display());
    let svg_data = match fs::read(&svg) {
        Ok(d) => d,
        Err(_) => return,
    };
    let opt = usvg::Options::default();
    let tree = match usvg::Tree::from_data(&svg_data, &opt) {
        Ok(t) => t,
        Err(_) => return,
    };
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let sizes = [16u32, 24, 32, 48, 64, 128, 256];
    let mut icon_dir = ico::IconDir::new(ico::ResourceType::Icon);
    for sz in sizes {
        let mut pixmap = tiny_skia::Pixmap::new(sz, sz).unwrap();
        let ts = tiny_skia::Transform::from_scale(
            sz as f32 / tree.size().width(),
            sz as f32 / tree.size().height(),
        );
        resvg::render(&tree, ts, &mut pixmap.as_mut());
        let png = pixmap.encode_png().unwrap();
        let png_path = out_dir.join(format!("icon-{sz}.png"));
        fs::write(&png_path, &png).unwrap();
        let image_data = ico::IconImage::read_png(png.as_slice()).unwrap();
        icon_dir.add_entry(ico::IconDirEntry::encode(&image_data).unwrap());
    }
    let ico_path = out_dir.join("mdview.ico");
    let mut ico_file = fs::File::create(&ico_path).unwrap();
    icon_dir.write(&mut ico_file).unwrap();
    #[cfg(windows)]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon(ico_path.to_str().unwrap());
        let _ = res.compile();
    }
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
