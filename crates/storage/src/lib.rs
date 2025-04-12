#![warn(clippy::pedantic)]

pub mod cached_rest;
pub mod indexed_db;
pub mod local_storage;
pub mod rest;

#[cfg(test)]
mod tests;
