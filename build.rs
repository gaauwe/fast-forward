use std::process::Command;
use std::path::Path;

fn main() {
    std::fs::create_dir_all("./.build").expect("Failed to create build directory");
    let swift_binary = Path::new("./.build").join("fast-forward-monitor");

    // Compile Swift code
    let status = Command::new("swiftc")
        .arg("./swift-lib/lib.swift")
        .arg("-o")
        .arg(&swift_binary)
        .status()
        .expect("Failed to compile Swift code");

    if !status.success() {
        panic!("Failed to compile Swift code");
    }
}
