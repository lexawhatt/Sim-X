use crate::{BodyDesc, BodyId, CollisionShape, Error, MAX_BODIES, Mobility, NumericStage, numeric};

/// Optional angular state. Without a record a legacy body has locked orientation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RotationDesc {
    /// Existing stable body identity, unique among rotational records.
    pub body: BodyId,
    /// Counterclockwise body angle in radians, in `-1e6..=1e6`.
    pub angle_rad: f64,
    /// Angular velocity in radians/second, in `-1e6..=1e6`.
    pub angular_velocity_rad_s: f64,
    /// Positive planar moment of inertia in `1e-15..=1e24` kg*m^2.
    pub inertia_kg_m2: f64,
}

impl RotationDesc {
    /// Computes uniform planar disk or rectangular-lamina inertia. Shape angle
    /// does not affect inertia; `angle_rad` is the separate initial body angle.
    /// This does not assume the inertia of a three-dimensional sphere.
    pub fn uniform(
        body: BodyDesc,
        shape: CollisionShape,
        angle_rad: f64,
        angular_velocity_rad_s: f64,
    ) -> Result<Self, Error> {
        body.validate()?;
        if !crate::collision::model::valid_shape(shape) {
            return Err(Error::InvalidRotation(body.id));
        }
        let inertia_kg_m2 = match shape {
            CollisionShape::Circle { radius_m } => numeric::product(
                &[0.5, body.mass_kg, radius_m, radius_m],
                NumericStage::Telemetry,
            )?,
            CollisionShape::Box { half_extents_m, .. } => {
                let x = numeric::product(
                    &[body.mass_kg / 3.0, half_extents_m.x, half_extents_m.x],
                    NumericStage::Telemetry,
                )?;
                let y = numeric::product(
                    &[body.mass_kg / 3.0, half_extents_m.y, half_extents_m.y],
                    NumericStage::Telemetry,
                )?;
                numeric::sum(&[x, y], NumericStage::Telemetry)?
            }
        };
        let result = Self {
            body: body.id,
            angle_rad,
            angular_velocity_rad_s,
            inertia_kg_m2,
        };
        result.validate(body)?;
        Ok(result)
    }

    pub(crate) fn validate(self, body: BodyDesc) -> Result<(), Error> {
        if self.body != body.id
            || !(-1e6..=1e6).contains(&self.angle_rad)
            || !(-1e6..=1e6).contains(&self.angular_velocity_rad_s)
            || !(1e-15..=1e24).contains(&self.inertia_kg_m2)
            || (body.mobility == Mobility::Fixed && self.angular_velocity_rad_s != 0.0)
        {
            return Err(Error::InvalidRotation(self.body));
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub(crate) struct Rotations {
    pub states: Vec<RotationDesc>,
    pub indices: Vec<Option<usize>>,
}

impl Rotations {
    pub fn bind(bodies: &[BodyDesc], input: &[RotationDesc]) -> Result<Self, Error> {
        if input.len() > MAX_BODIES {
            return Err(Error::RotationBudget);
        }
        let mut states = input.to_vec();
        states.sort_by_key(|state| state.body);
        for pair in states.windows(2) {
            if pair[0].body == pair[1].body {
                return Err(Error::DuplicateRotation(pair[0].body));
            }
        }
        let mut indices = vec![None; bodies.len()];
        for (index, &state) in states.iter().enumerate() {
            let body = bodies
                .binary_search_by_key(&state.body, |body| body.id)
                .map_err(|_| Error::MissingBody(state.body))?;
            state.validate(bodies[body])?;
            indices[body] = Some(index);
        }
        Ok(Self { states, indices })
    }

    pub fn get(&self, body_index: usize) -> Option<RotationDesc> {
        self.indices[body_index].map(|index| self.states[index])
    }

    pub fn angle(&self, body_index: usize) -> f64 {
        self.get(body_index).map_or(0.0, |state| state.angle_rad)
    }
    pub fn omega(&self, body_index: usize) -> f64 {
        self.get(body_index)
            .map_or(0.0, |state| state.angular_velocity_rad_s)
    }
    pub fn inverse_inertia(&self, body_index: usize, bodies: &[BodyDesc]) -> f64 {
        if bodies[body_index].mobility == Mobility::Fixed {
            0.0
        } else {
            self.get(body_index)
                .map_or(0.0, |state| 1.0 / state.inertia_kg_m2)
        }
    }
}
