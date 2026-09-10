//! Sim;X application and scientific domain library.
//!
//! The current scientific milestone contains four independent `Sim;Phys`
//! slices: translational Mechanics, lumped-conduction Thermodynamics, a 1D
//! wave field, and stationary electrostatic field evaluation. Presentation and
//! desktop lifecycle remain optional so every scientific core runs headlessly.

#[cfg(feature = "desktop")]
/// Desktop application composition and startup boundary.
pub mod app;
pub mod domains;
pub mod foundation;
#[cfg(feature = "desktop")]
pub(crate) mod presentation;
