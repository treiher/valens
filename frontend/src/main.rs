#![warn(clippy::pedantic)]
#![allow(
    clippy::match_wildcard_for_single_variants,
    clippy::must_use_candidate,
    clippy::too_many_lines,
    clippy::wildcard_imports
)]

mod domain;
mod storage;
mod ui;

fn main() {
    ui::main();
}
