#![deny(clippy::pedantic)]

fn main() {
    let version = std::env::var("VALENS_VERSION").unwrap_or("dev".to_string());
    println!("cargo:rustc-env=VALENS_VERSION={version}");
}
