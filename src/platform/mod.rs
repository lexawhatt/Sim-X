//! Native services outside the scientific and menu state models.

mod clipboard;
mod links;

pub use clipboard::install_clipboard;
pub use links::install_link_opener;
