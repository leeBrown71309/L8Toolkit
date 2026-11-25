//! Spotlight search: indexes, ranking and result building.
//!
//! This module is deliberately free of Tauri types so it can be unit-tested and
//! reused by future widgets.

pub mod apps;
pub mod calculator;
pub mod files;
mod matching;
mod service;
pub mod system_actions;
pub mod types;
pub mod web;

pub use service::{SearchService, UsageInfo};
pub use types::{ResultKind, SearchResult, SearchSection};
