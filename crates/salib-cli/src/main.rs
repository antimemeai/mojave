//! `saltelli` binary entry point.
//!
//! Pre-code stub (2026-04-28). The actual subcommand surface lands in
//! Phase B+ of `plans/0002-saltelli-roadmap.md`. This stub exists so the
//! workspace builds and `cargo xtask ci` is green over the saltelli scaffold.
//!
//! See `crates/salib-cli/src/lib.rs` for the planned subcommand surface
//! and `decisions/2026-04-28-saltelli-where-and-naming.md` for the crate
//! split that puts the binary here.

#![forbid(unsafe_code)]

fn main() {
    eprintln!("saltelli: pre-code scaffold (2026-04-28); see plans/0002-saltelli-roadmap.md");
    std::process::exit(2);
}
