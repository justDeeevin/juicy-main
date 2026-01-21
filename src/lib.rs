//! Juicy fn main in rust.
//!
//! Inspired by [Zig's juicy main functionality](https://codeberg.org/ziglang/zig/pulls/30644),
//! this crate provides an attribute to allow the main function of a binary crate to have input
//! parameters for environment variables and command-line arguments.
//!
//! # Usage
//!
//! Adding the `juicy` attribute to `fn main` will allow the function to accept up to two
//! parameters. Whether each parameter is for env vars or args is inferred from their types.
//!
//! Environment variables can be provided as
//! - `&[(String, String)]`, a slice of key-value pairs[^1]
//! - [`std::env::Vars`], an iterator of key-value pairs
//! - `Vec<(String, String)>`, a vector of key-value pairs
//! - `HashMap<String, String>`, a hash map of key-value pairs
//!
//! Command-line arguments can be provided as
//! - `&[String]`, a slice of strings[^1]
//! - [`std::env::Args`], an iterator of strings
//! - `Vec<String>`, a vector of strings
//! - (with the `clap` feature enabled) any struct implementing [`clap::Parser`], which will
//!   automatically be parsed
//!
//! This inference is based on **the identifier of the type**, so type aliases or name collisions
//! will cause improper behavior.
//!
//! # Example
//!
//! ```rust
//! #[juicy_main::juicy]
//! fn main(env: HashMap<String, String>, args: Vec<String>) {
//!     dbg!(env);
//!     eprintln!("executable: {}", args[0]);
//! }
//! ```
//!
//! There is an example using [clap](clap) in the `examples` directory.
//!
//! [^1]: This doesn't avoid allocations, merely providing a reference to an obscured collected
//! [`Vec`].

#[cfg(feature = "clap")]
#[doc(hidden)]
/// Re-export of `clap::Parser` for use in expanded macro.
pub mod clap;

pub use juicy_main_macro::juicy;
