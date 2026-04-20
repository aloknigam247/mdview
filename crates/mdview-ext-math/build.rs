//! Ensure the KaTeX vendor bundle is present under `assets/vendor/`.
//!
//! If the files are already committed (the usual case), this is a no-op.
//! Otherwise we try to fetch the official release zip once and cache it
//! under `target/mdv-katex-cache`. Network failure is non-fatal — the
//! extension ships stub files so the build never breaks offline.

use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

const KATEX_VERSION: &str = "0.16.11";

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=assets/vendor/katex.min.js");
    println!("cargo:rerun-if-changed=assets/vendor/katex.min.css");

    let assets_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("assets")
        .join("vendor");
    if let Err(e) = fs::create_dir_all(&assets_dir) {
        println!("cargo:warning=mdview-ext-math: cannot create assets dir: {e}");
        return;
    }

    let js = assets_dir.join("katex.min.js");
    let css = assets_dir.join("katex.min.css");
    let init = assets_dir.join("mdv-math-init.js");

    if !init.exists() {
        let body = "(function(){\n  if (window.renderMathInElement) {\n    window.renderMathInElement(document.body, {delimiters:[{left:'\\\\[', right:'\\\\]', display:true},{left:'\\\\(', right:'\\\\)', display:false}]});\n  }\n})();\n";
        let _ = fs::write(&init, body);
    }

    let have_js = file_nonempty(&js);
    let have_css = file_nonempty(&css);
    if have_js && have_css {
        return;
    }

    let cache_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap_or_else(|_| ".".to_string()))
        .join("mdv-katex-cache");
    let _ = fs::create_dir_all(&cache_dir);

    if try_fetch(&cache_dir, &js, &css).is_err() {
        if !have_js {
            let _ = fs::write(
                &js,
                b"// KaTeX offline stub - network fetch failed at build time.\n\
                  // Replace with the real katex.min.js before shipping.\n\
                  window.renderMathInElement = function(){};\n",
            );
        }
        if !have_css {
            let _ = fs::write(
                &css,
                b"/* KaTeX offline stub - replace with real katex.min.css */\n",
            );
        }
        println!(
            "cargo:warning=mdview-ext-math: could not fetch KaTeX {} — using offline stubs",
            KATEX_VERSION
        );
    }
}

fn file_nonempty(p: &Path) -> bool {
    fs::metadata(p).map(|m| m.len() > 0).unwrap_or(false)
}

fn try_fetch(_cache: &Path, js: &Path, css: &Path) -> io::Result<()> {
    let js_url = format!(
        "https://cdn.jsdelivr.net/npm/katex@{}/dist/katex.min.js",
        KATEX_VERSION
    );
    let css_url = format!(
        "https://cdn.jsdelivr.net/npm/katex@{}/dist/katex.min.css",
        KATEX_VERSION
    );

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .map_err(|e| io::Error::other(e.to_string()))?;

    fetch_to(&client, &js_url, js)?;
    fetch_to(&client, &css_url, css)?;
    Ok(())
}

fn fetch_to(client: &reqwest::blocking::Client, url: &str, dest: &Path) -> io::Result<()> {
    let mut resp = client
        .get(url)
        .send()
        .map_err(|e| io::Error::other(e.to_string()))?;
    if !resp.status().is_success() {
        return Err(io::Error::other(format!("HTTP {}", resp.status())));
    }
    let mut buf = Vec::new();
    resp.read_to_end(&mut buf)?;
    fs::write(dest, buf)?;
    Ok(())
}
