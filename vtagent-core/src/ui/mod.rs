//! User interface utilities and shared UI components
//!
//! This module contains shared UI functionality including loading indicators,
//! markdown rendering, and terminal utilities.

pub mod markdown;
pub mod spinner;
pub mod terminal;
pub mod user_confirmation;
pub mod diff_renderer;

pub use markdown::*;
pub use spinner::*;
pub use terminal::*;
