#![deny(clippy::pedantic)]

use std::process::Command;

fn main() {
    let output = Command::new("python")
        .args([
            "-c",
            "from setuptools_scm import get_version; print(get_version(root='..'))",
        ])
        .output()
        .unwrap();
    let version = String::from_utf8(output.stdout).unwrap();
    println!("cargo:rustc-env=VALENS_VERSION={version}");
}
