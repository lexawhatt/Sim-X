//! Short UI easing only. Never interpolate calculated measurements or topology.
use crate::math_editor::{state::MathState, view::layout::Target};
#[derive(Default)]
pub(in crate::math_editor) struct Motion {
    hover: Vec<(Target, f32)>,
    pub popup: f32,
    pub plot_opacity: f32,
}
impl Motion {
    pub fn hover(&self, target: Target) -> f32 {
        self.hover
            .iter()
            .find(|(t, _)| *t == target)
            .map_or(0.0, |(_, v)| *v)
    }
}
impl MathState {
    pub(in crate::math_editor) fn animate_ui(&mut self, dt: f32) {
        let t = if self.reduced_motion {
            1.0
        } else {
            1.0 - (-dt / 0.09).exp()
        };
        if let Some(target) = self.hovered
            && !self.motion.hover.iter().any(|(old, _)| *old == target)
        {
            self.motion.hover.push((target, 0.0));
        }
        for (target, value) in &mut self.motion.hover {
            *value += (f32::from(self.hovered == Some(*target)) - *value) * t;
        }
        self.motion.hover.retain(|(_, v)| *v > 0.001);
        if self.functions || self.style_popup.is_some() {
            self.motion.popup += (1.0 - self.motion.popup) * t;
        } else {
            self.motion.popup = 0.0;
        }
        let opacity = if self.stale && self.parameter_drag.is_none() && self.playback.is_none() {
            0.55
        } else {
            1.0
        };
        self.motion.plot_opacity += (opacity - self.motion.plot_opacity) * t;
    }
}

impl MathState {
    pub(in crate::math_editor) fn advance_integral_reveal(&mut self, dt: f32) {
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
