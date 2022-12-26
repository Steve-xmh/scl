#![doc = include_str!("../README.md")]
#![forbid(missing_docs)]

pub mod auth;
pub mod client;
pub mod download;
pub mod http;
pub mod java;
pub mod modpack;
pub mod password;
pub mod progress;
pub mod semver;
pub mod utils;
pub mod version;

pub(crate) mod package;
pub(crate) mod path;
pub(crate) mod prelude;
