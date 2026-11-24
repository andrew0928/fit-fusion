use std::{env, process::Command};

fn main() {
    println!("cargo::rerun-if-changed=../build_info.rs");
    let compiler = Command::new(env::var_os("RUSTC").expect("Cargo sets RUSTC"))
        .arg("--version")
        .output()
        .expect("could not query the Rust compiler");
    assert!(compiler.status.success(), "rustc --version failed");
    let version = String::from_utf8(compiler.stdout).expect("rustc version is UTF-8");
    println!("cargo::rustc-env=FIT_FUSION_COMPILER={}", version.trim());
    println!(
        "cargo::rustc-env=FIT_FUSION_TARGET={}",
        env::var("TARGET").expect("Cargo sets TARGET")
    );
}
