use std::process::Command;

fn main() {
    let status = Command::new("swift")
        .arg("build")
        .arg("--package-path")
        .arg("./swift-lib")
        .arg("--configuration")
        .arg("release")
        .status()
        .expect("Failed to build Swift package");

    if !status.success() {
        panic!("Failed to build Swift package");
    }

    prost_build::compile_protos(&["src/proto/socket.proto"], &["src/proto"]).unwrap();
}
