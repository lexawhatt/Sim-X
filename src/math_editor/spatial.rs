//! Orthographic orbit presentation using Logic's public Engine camera.
//! This first slice is deliberately wireframe: no hidden-surface claims.
use super::layout::{Camera, Rect};
use sim_logic::prelude::*;

pub(super) const HOME_ANGLES: [f64; 2] = [std::f64::consts::FRAC_PI_4, std::f64::consts::PI / 6.0];
const NEAR: f32 = 0.01;
const FAR: f32 = 128.0;

pub(super) struct Spatial {
    pub enabled: bool,
    pub blend: f64,
    pub yaw: f64,
    pub pitch: f64,
    pub target: [f64; 2],
    pub drag: Option<([f32; 2], [f64; 2])>,
}
impl Default for Spatial {
    fn default() -> Self {
        Self {
            enabled: false,
            blend: 0.0,
            yaw: HOME_ANGLES[0],
            pitch: HOME_ANGLES[1],
            target: HOME_ANGLES,
            drag: None,
        }
    }
}
impl Spatial {
    pub fn visible(&self) -> bool {
        self.enabled || self.blend > 0.0
    }
    pub fn advance(&mut self, dt: f64, reduced: bool) {
        let t = if reduced {
            1.0
        } else {
            1.0 - (-dt / 0.16).exp()
        };
        let goal = f64::from(self.enabled);
        self.blend += (goal - self.blend) * t;
        if (goal - self.blend).abs() < 0.0001 {
            self.blend = goal;
        }
        self.yaw += (self.target[0] - self.yaw) * t;
        self.pitch += (self.target[1] - self.pitch) * t;
    }
    pub fn orbit(&mut self, position: [f32; 2]) {
        if let Some((origin, angles)) = self.drag {
            self.target = [
                angles[0] - f64::from(position[0] - origin[0]) * 0.007,
                (angles[1] + f64::from(position[1] - origin[1]) * 0.007).clamp(-1.35, 1.35),
            ];
        }
    }
    pub fn projector(&self, plane: Camera, rect: Rect) -> LogicResult<Projector> {
        let viewport = LogicalViewport::new(rect.w, rect.h)?;
        let yaw = self.yaw * self.blend;
        let pitch = self.pitch * self.blend;
        let eye = Vec3::new(
            (yaw.sin() * pitch.cos() * 64.0) as f32,
            (pitch.sin() * 64.0) as f32,
            (yaw.cos() * pitch.cos() * 64.0) as f32,
        )?;
        let mut view = View3d::new(eye, Vec3::ZERO)?;
        // Normalize around the scientific camera in f64 first. This keeps a
        // large coordinate offset out of the renderer's f32 look-at basis.
        view.set_orthographic(
            WorldLength::new(2.0)?,
            WorldLength::new(NEAR)?,
            WorldLength::new(FAR)?,
        )?;
        let (sine, cosine) = (std::f64::consts::FRAC_PI_2 * self.blend).sin_cos();
        Ok(Projector {
            camera: view.camera(viewport)?,
            forward: Vec3::ZERO.checked_sub(eye)?.normalized()?,
            plane,
            rect,
            viewport,
            basis: [sine, cosine],
        })
    }
}
pub(super) struct Projector {
    camera: Camera3d,
    forward: Vec3,
    plane: Camera,
    rect: Rect,
    viewport: LogicalViewport,
    basis: [f64; 2],
}
impl Projector {
    pub fn point(&self, p: [f64; 3]) -> Option<[f32; 2]> {
        let point = self.normalized(p)?;
        let depth = self.view_depth(point);
        if !(f64::from(NEAR)..=f64::from(FAR)).contains(&depth) {
            return None;
        }
        self.project(point)
    }

    pub fn depth(&self, p: [f64; 3]) -> Option<f64> {
        Some(self.view_depth(self.normalized(p)?))
    }

    pub fn segment(&self, points: [[f64; 3]; 2]) -> Option<ProjectedSegment> {
        let a = self.normalized(points[0])?;
        let b = self.normalized(points[1])?;
        let (points, depths) = super::spatial_clip::slab(
            [a, b],
            [self.view_depth(a), self.view_depth(b)],
            [f64::from(NEAR), f64::from(FAR)],
        )?;
        Some(ProjectedSegment {
            points: [self.project(points[0])?, self.project(points[1])?],
            depths,
        })
    }

    fn normalized(&self, p: [f64; 3]) -> Option<[f64; 3]> {
        let factor = self.plane.scale * 2.0 / f64::from(self.rect.h);
        let v = [
            (p[0] - self.plane.x) * factor,
            (p[1] - self.plane.y) * factor,
            p[2] * factor,
        ];
        if !v
            .iter()
            .all(|x| x.is_finite() && x.abs() < f64::from(f32::MAX))
        {
            return None;
        }
        // Logic's camera is renderer-Y-up. Mathematical Z becomes that up axis
        // through a rigid display-basis rotation, never by changing samples or
        // swapping a formula's variables. At blend=0 the XY view is identical.
        let [sine, cosine] = self.basis;
        Some([
            v[0],
            v[1] * cosine + v[2] * sine,
            -v[1] * sine + v[2] * cosine,
        ])
    }

    fn view_depth(&self, point: [f64; 3]) -> f64 {
        let eye = self.camera.position();
        let forward = self.forward;
        (point[0] - f64::from(eye.x())) * f64::from(forward.x())
            + (point[1] - f64::from(eye.y())) * f64::from(forward.y())
            + (point[2] - f64::from(eye.z())) * f64::from(forward.z())
    }

    fn project(&self, point: [f64; 3]) -> Option<[f32; 2]> {
        let point = Vec3::new(point[0] as f32, point[1] as f32, point[2] as f32).ok()?;
        let projected = self.camera.project_world(point, self.viewport).ok()?;
        let xy = projected.logical_position().to_vec2();
        Some([self.rect.x + xy.x(), self.rect.y + xy.y()])
    }
}

pub(super) struct ProjectedSegment {
    pub points: [[f32; 2]; 2],
    pub depths: [f64; 2],
}
