//! Each completed row owns its reveal clock. One finished integral must not
//! advance another row's animation while that row is still calculating.
use super::state::MathState;

impl MathState {
    pub(super) fn advance_integral_reveal(&mut self, dt: f32) {
        self.area_progress.resize(self.document.fields.len(), 0.0);
        for (index, progress) in self.area_progress.iter_mut().enumerate() {
            let ready = !self.stale
                && self
                    .plot
                    .as_ref()
                    .and_then(|p| p.rows.get(index))
                    .is_some_and(|r| r.scalar.is_some() && !r.pending && r.integral_plot.is_some());
            if !ready {
                *progress = 0.0;
                continue;
            }
            *progress = if self.reduced_motion
                || self.playback.is_some()
                || self.parameter_drag.is_some()
            {
                1.0
            } else {
                (*progress + dt / 0.8).min(1.0)
            };
        }
    }
}
