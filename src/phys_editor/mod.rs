//! Physics authoring and an independent fixed-step View simulation instance.

pub(crate) mod app;
pub(crate) mod assets;
mod attachment;
#[cfg(test)]
mod attachment_input_tests;
mod catalog;
mod catalog_icons;
mod catalog_input;
mod catalog_layout;
mod catalog_view;
mod commands;
mod document;
pub(crate) mod input;
mod layout;
mod placement;
mod selection;
mod session;
pub(crate) mod state;
mod view;

pub use app::build_phys_editor_application;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod physics_tests;

#[cfg(test)]
mod attachment_runtime_tests;

#[cfg(test)]
mod catalog_tests;

#[cfg(test)]
mod selection_tests;
