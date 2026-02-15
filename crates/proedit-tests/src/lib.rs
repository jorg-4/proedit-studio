//! Integration test crate for ProEdit Studio.
//!
//! This crate exists solely to hold cross-crate integration tests.
//! It depends on multiple proedit crates to verify they work together.

#[cfg(test)]
mod timeline;

#[cfg(test)]
mod audio;

#[cfg(test)]
mod gpu;
