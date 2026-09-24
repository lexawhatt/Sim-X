//! Math-only authoring, structured formula input and presentation.
pub(crate) mod app;
mod compute;
mod formula;
pub(crate) mod interaction;
mod scene;
pub(crate) mod state;
mod view;

#[cfg(test)]
mod tests;

pub use app::build_math_editor_application;
