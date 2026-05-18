#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

mod atoms;
mod walk;

pub use atoms::{Casing, FormatAtoms, Padding, Punctuation, Separator};
pub use walk::{apply_atoms, longest_string_region, ApplyError};
