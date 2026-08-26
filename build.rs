//! Builds the dashboard UI (ui/ -> ui/dist) so it can be embedded into the
//! binary by `static_serve::embed_assets!` in main.rs.
//!
//! Set `LAWS_SKIP_UI_BUILD=1` to skip running npm (e.g. in Docker, where the
//! UI is built in a separate stage and copied into ui/dist beforehand).

use std::path::Path;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-env-changed=LAWS_SKIP_UI_BUILD");
    println!("cargo:rerun-if-changed=ui/index.html");
    println!("cargo:rerun-if-changed=ui/src");
    println!("cargo:rerun-if-changed=ui/package-lock.json");
    println!("cargo:rerun-if-changed=ui/vite.config.ts");

    let dist = Path::new("ui/dist");

    if std::env::var_os("LAWS_SKIP_UI_BUILD").is_some() {
        if !dist.join("index.html").exists() {
            // embed_assets! needs the directory to exist; ship a stub so the
            // binary still builds without a dashboard.
            std::fs::create_dir_all(dist.join("assets")).expect("create ui/dist/assets");
            std::fs::write(
                dist.join("index.html"),
                "<!doctype html><title>laws</title><p>Dashboard UI was not built (LAWS_SKIP_UI_BUILD was set).</p>",
            )
            .expect("write ui/dist/index.html");
        }
        return;
    }

    // On Windows, npm is a batch script (npm.cmd); `Command::new("npm")`
    // only resolves executables, so it would fail with "program not found".
    let npm = if cfg!(windows) { "npm.cmd" } else { "npm" };
    run(npm, &["ci", "--ignore-scripts"]);
    run(npm, &["run", "build"]);
}

fn run(program: &str, args: &[&str]) {
    let status = Command::new(program).args(args).current_dir("ui").status();
    match status {
        Ok(s) if s.success() => {}
        Ok(s) => panic!("`{program} {}` failed with {s}", args.join(" ")),
        Err(e) => panic!(
            "failed to run `{program} {}`: {e}\n\
             Building laws requires Node.js/npm to compile the dashboard UI.\n\
             Install Node.js, or set LAWS_SKIP_UI_BUILD=1 to build without the dashboard.",
            args.join(" ")
        ),
    }
}
