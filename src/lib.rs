//! The new Sim;X application shell, hosted by Sim;Logic.
//!
//! Includes the main menu, domain/scale pickers and named session projects in
//! a Physics editor with a separately owned, renderer-independent mechanics run.
//! The physical model includes 2D bodies, rods, attached springs and gravity.
//! Math has its own structured expression editor and renderer-independent core.

#![warn(missing_docs)]

mod actions;
mod app;
mod math_editor;
mod menu;
mod navigation;
mod phys_editor;
#[cfg(feature = "desktop")]
pub mod platform;

pub use app::build_application;
pub use math_editor::build_math_editor_application;
pub use menu::input::MenuAction;
pub use phys_editor::build_phys_editor_application;
pub use phys_editor::input::EditorAction;
