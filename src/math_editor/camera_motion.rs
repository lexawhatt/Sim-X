//! Camera-only easing. Scientific values and sampled geometry are not morphed.
use super::{
    layout::{Camera, Rect},
    state::MathState,
};

impl MathState {
    pub(super) fn zoom_view(&mut self, factor: f64, point: [f32; 2], rect: Rect) {
        let mut target = self.camera_target.unwrap_or(self.camera);
        target.zoom(
            factor,
            if self.spatial.visible() {
                rect.center()
            } else {
                point
            },
            rect,
        );
        self.camera_target = Some(target);
        self.dirty = true;
    }
    pub(super) fn reset_view(&mut self) {
        self.camera_target = Some(Camera::default());
        self.pan = None;
        self.spatial.drag = None;
        let home = super::spatial::HOME_ANGLES;
        let difference = (home[0] - self.spatial.yaw + std::f64::consts::PI)
            .rem_euclid(std::f64::consts::TAU)
            - std::f64::consts::PI;
        self.spatial.target = [self.spatial.yaw + difference, home[1]];
        self.dirty = true;
    }
    pub(super) fn advance_camera(&mut self, dt: f64) {
        let spatial_was_visible = self.spatial.visible();
        self.spatial.advance(dt, self.reduced_motion);
        if spatial_was_visible && !self.spatial.visible() {
            self.dirty = true;
        }
        let Some(target) = self.camera_target else {
            return;
        };
        let t = if self.reduced_motion {
            1.0
        } else {
            1.0 - (-dt / 0.10).exp()
        };
        // Linear interpolation of reciprocal scale preserves the wheel's
        // anchored world point throughout the zoom, not only at its endpoint.
        let scale = 1.0 / ((1.0 - t) / self.camera.scale + t / target.scale);
        let x = self.camera.x * (1.0 - t) + target.x * t;
        let y = self.camera.y * (1.0 - t) + target.y * t;
        if !scale.is_finite() || scale <= 0.0 || !x.is_finite() || !y.is_finite() {
            self.camera_target = None;
            return;
        }
        self.camera = Camera { x, y, scale };
        let remaining = ((target.x - x).hypot(target.y - y) * scale)
            .max((scale / target.scale - 1.0).abs() * 1000.0);
        if self.reduced_motion || remaining < 0.01 {
            self.camera = target;
            self.camera_target = None;
        }
    }
}
