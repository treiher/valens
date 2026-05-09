#![deny(clippy::pedantic)]

fn main() {
    println!("cargo:rerun-if-env-changed=VALENS_VERSION");
    let version = std::env::var("VALENS_VERSION").unwrap_or("dev".to_string());
    println!("cargo:rustc-env=VALENS_VERSION={version}");
}
