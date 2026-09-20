//! Math-only authoring, structured formula input and presentation.
pub(crate) mod app;
mod assets;
mod axis_spatial;
mod axis_ticks;
mod axis_view;
mod camera_motion;
pub(crate) mod clipboard;
mod commands;
mod contour_display;
mod curve_display;
mod drawing;
mod editing;
mod entry_hint;
mod formula;
mod formula_import;
mod formula_layout;
mod formula_selection;
mod graph_style;
mod graph_view;
mod input;
mod integral_motion;
mod integral_plot;
mod integral_view;
mod keyboard_layout;
mod layout;
mod motion;
mod parameters;
mod point_labels;
mod row_flow;
mod row_interpretation;
mod sidebar_view;
mod spatial;
mod spatial_clip;
mod spatial_view;
pub(crate) mod state;
mod surface_paths;
mod view;
mod wireframe_view;
mod worker;
mod worker_calculus;
mod worker_spatial;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod visual_probe;

pub use app::build_math_editor_application;
