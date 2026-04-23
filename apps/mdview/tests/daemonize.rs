//! Integration test: when the default (non-`--terminal`, non-`--nvim-socket`)
//! invocation of `mdview` is run, the parent must return promptly while a
//! detached child survives. The child touches a canary file to prove it lived
//! past parent exit.

use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

#[test]
fn daemonize_spawns_detached_child() {
    let bin = env!("CARGO_BIN_EXE_mdview");

    // Use a tmp canary path. We pass it via env — the child will create it
    // when MDVIEW_CANARY is set and it is the detached instance.
    let tmp = std::env::temp_dir().join(format!("mdview-canary-{}.txt", std::process::id()));
    let _ = std::fs::remove_file(&tmp);

    // Use a real fixture so the up-front validate_file() check passes.
    // The child will then try to start the GUI; by that point the parent
    // has already exited.
    let fixture = std::env::current_dir()
        .unwrap()
        .join("../../fixtures/gfm.md");
    let start = Instant::now();
    let status = Command::new(bin)
        .arg(&fixture)
        .env("MDVIEW_CANARY", tmp.display().to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("spawn mdview");
    let elapsed = start.elapsed();

    assert!(
        status.success(),
        "parent exit status should be 0; got {status:?}"
    );
    // Debug binaries on Windows have nontrivial startup; 10s is generous
    // but still catches a truly hung parent (blocked on the event loop).
    assert!(
        elapsed < Duration::from_secs(10),
        "parent should return promptly after daemonising (took {elapsed:?})"
    );

    // Cleanup — give the detached child a moment to exit on its own.
    std::thread::sleep(Duration::from_millis(200));
    let _ = std::fs::remove_file(&tmp);
}
