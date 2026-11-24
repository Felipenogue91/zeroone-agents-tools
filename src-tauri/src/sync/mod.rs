//! Multi-CLI sync (Claude Code, Codex, Antigravity CLI).
//!
//! `~/.agents` is the source of truth; this module reads it, models every
//! resource with its per-CLI target matrix, and (in later phases) plans and
//! applies the generated CLI files.

pub mod codex;
pub mod engine;
pub mod model;
pub mod occurrences;
pub mod secrets;
pub mod skills;
pub mod source;
