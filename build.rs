//! Embeds the dashboard UI (ui/dist) into the binary via
//! `static_serve::embed_assets!` in main.rs.
//!
//! `ui/dist` is committed to the repository so that `cargo build`,
//! `cargo install`, cross-compilation and Docker all work without Node.js.
//! When npm is available (and `LAWS_SKIP_UI_BUILD` is not set) the UI is
//! rebuilt from `ui/src` so local changes are picked up; the result should be
//! committed alongside UI source changes (CI verifies this).

use std::path::Path;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-env-changed=LAWS_SKIP_UI_BUILD");
    println!("cargo:rerun-if-changed=ui/index.html");
    println!("cargo:rerun-if-changed=ui/src");
    println!("cargo:rerun-if-changed=ui/package-lock.json");
    println!("cargo:rerun-if-changed=ui/vite.config.ts");

    let dist = Path::new("ui/dist");
    let has_dist = dist.join("index.html").exists();

    // On Windows, npm is a batch script (npm.cmd); `Command::new("npm")`
    // only resolves executables, so it would fail with "program not found".
    let npm = if cfg!(windows) { "npm.cmd" } else { "npm" };

    if std::env::var_os("LAWS_SKIP_UI_BUILD").is_some() {
        if !has_dist {
            write_stub(dist, "LAWS_SKIP_UI_BUILD was set");
        }
        return;
    }

    let npm_available = Command::new(npm)
        .arg("--version")
        .current_dir("ui")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if npm_available {
        run(npm, &["ci", "--ignore-scripts"]);
        run(npm, &["run", "build"]);
    } else if has_dist {
        println!("cargo:warning=npm not found; embedding the prebuilt dashboard from ui/dist");
    } else {
        write_stub(dist, "npm was not found");
        println!(
            "cargo:warning=npm not found and ui/dist is missing; the dashboard UI will be a stub. \
             Install Node.js to build it."
        );
    }
}

fn write_stub(dist: &Path, reason: &str) {
    // embed_assets! needs the directory to exist; ship a stub so the binary
    // still builds without a dashboard.
    std::fs::create_dir_all(dist.join("assets")).expect("create ui/dist/assets");
    std::fs::write(
        dist.join("index.html"),
        format!("<!doctype html><title>laws</title><p>Dashboard UI was not built ({reason}).</p>"),
    )
    .expect("write ui/dist/index.html");
}

fn run(program: &str, args: &[&str]) {
    let status = Command::new(program).args(args).current_dir("ui").status();
    match status {
        Ok(s) if s.success() => {}
        Ok(s) => panic!("`{program} {}` failed with {s}", args.join(" ")),
        Err(e) => panic!("failed to run `{program} {}`: {e}", args.join(" ")),
    }
}
