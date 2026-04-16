fn main() {
    // Don't want just ui, as then the build output would invalidate
    // it every run. This isn't complete, but it's probably fine for dev.
    println!("cargo:rerun-if-changed=ui/index.html");
    println!("cargo:rerun-if-changed=ui/src");
    println!("cargo:rerun-if-changed=ui/package-lock.json");

    let status = std::process::Command::new("npm")
        .arg("ci")
        .arg("--ignore-scripts")
        .current_dir("ui")
        .status()
        .unwrap();
    assert!(status.success(), "npm ci failed: {}", status);

    let status = std::process::Command::new("npm")
        .arg("run")
        .arg("build")
        .current_dir("ui")
        .status()
        .unwrap();
    assert!(status.success(), "npm run build failed: {}", status);
}
