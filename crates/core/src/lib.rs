//! Datum Layout engine: the plant model and its edits, the generated parts catalogue, the six
//! parametric equipment types, pipe routing (straights, spec elbows, reducers, flanges, snapping
//! components), clash and spec checks, lists and material take-off, and PCF and glTF exports.
//! Pure Rust; the window only shows what this computes.

#![allow(clippy::too_many_arguments, clippy::type_complexity, clippy::needless_range_loop, clippy::manual_is_multiple_of)]

pub mod catalogue;
pub mod checks;
pub mod doc;
pub mod equipment;
pub mod examples;
pub mod gltf;
pub mod lists;
pub mod model;
pub mod package;
pub mod parts;
pub mod pcf;
pub mod route;
pub mod scene;
pub mod units;
#[cfg(test)]
mod geometry_tests;
#[cfg(test)]
mod exchange_tests;
#[cfg(test)]
mod hardening;
