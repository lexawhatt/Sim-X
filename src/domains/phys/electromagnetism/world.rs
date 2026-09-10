use std::collections::BTreeMap;

use crate::foundation::{
    EntityId, PhysicalConstants, WorldRevision,
    reproducible_sum::reproducible_sum,
    scaled_product::{ScaledProductError, scaled_product_quotient},
};

use super::{
    ElectricField2, ElectrostaticArithmetic, ElectrostaticError, ElectrostaticSnapshot,
    FieldProbeId, FieldProbeSnapshot, NewtonsPerCoulomb, PointChargeSnapshot, PointChargeSpec,
    Position2, model::PointCharge,
};

/// Hard maximum number of point charges in one electrostatic world.
pub const MAX_CHARGES: usize = 1_024;
/// Hard maximum number of field probes in one electrostatic world.
pub const MAX_FIELD_PROBES: usize = 1_024;
/// Hard pair-evaluation budget for one public `evaluate` call.
pub const MAX_ELECTROSTATIC_INTERACTIONS: usize = 262_144;

/// Canonical stationary point-charge and field-probe world.
///
/// Creation is atomic and bounded. Evaluation is pure and consumes an explicit
/// immutable constants registry; no field result is retained as canonical
/// state.
#[derive(Clone, Debug, PartialEq)]
pub struct ElectrostaticWorld {
    charges: BTreeMap<EntityId, PointCharge>,
    probes: BTreeMap<FieldProbeId, Position2>,
    next_charge_id: Option<u64>,
    next_probe_id: Option<u64>,
    revision: WorldRevision,
}

impl Default for ElectrostaticWorld {
    fn default() -> Self {
        Self::new()
    }
}

impl ElectrostaticWorld {
    /// Creates an empty stationary electrostatic world.
    pub fn new() -> Self {
        Self {
            charges: BTreeMap::new(),
            probes: BTreeMap::new(),
            next_charge_id: Some(1),
            next_probe_id: Some(1),
            revision: WorldRevision::ZERO,
        }
    }

    /// Creates a stationary point charge at a finite position in meters.
    ///
    /// Capacity, identity, and revision failure commit nothing.
    pub fn create_charge(&mut self, spec: PointChargeSpec) -> Result<EntityId, ElectrostaticError> {
        if self.charges.len() >= MAX_CHARGES {
            return Err(ElectrostaticError::ChargeCapacityReached {
                maximum: MAX_CHARGES,
            });
        }
        let raw_id = self
            .next_charge_id
            .ok_or(ElectrostaticError::ChargeIdentityExhausted)?;
        let revision = self
            .revision
            .checked_next()
            .ok_or(ElectrostaticError::CounterExhausted)?;
        let entity =
            EntityId::new(raw_id).map_err(|_| ElectrostaticError::ChargeIdentityExhausted)?;
        self.charges.insert(
            entity,
            PointCharge {
                entity,
                position: spec.position,
                charge: spec.charge,
            },
        );
        self.next_charge_id = raw_id.checked_add(1);
        self.revision = revision;
        Ok(entity)
    }

    /// Creates a finite field-probe position in world meters.
    ///
    /// A probe may be created at any position, but evaluation rejects it if a
    /// point charge occupies exactly the same location.
    pub fn create_probe(
        &mut self,
        position: Position2,
    ) -> Result<FieldProbeId, ElectrostaticError> {
        if self.probes.len() >= MAX_FIELD_PROBES {
            return Err(ElectrostaticError::ProbeCapacityReached {
                maximum: MAX_FIELD_PROBES,
            });
        }
        let raw_id = self
            .next_probe_id
            .ok_or(ElectrostaticError::ProbeIdentityExhausted)?;
        let revision = self
            .revision
            .checked_next()
            .ok_or(ElectrostaticError::CounterExhausted)?;
        let probe = FieldProbeId::new(raw_id).ok_or(ElectrostaticError::ProbeIdentityExhausted)?;
        self.probes.insert(probe, position);
        self.next_probe_id = raw_id.checked_add(1);
        self.revision = revision;
        Ok(probe)
    }

    /// Purely evaluates the Coulomb electric field at every probe.
    ///
    /// Formula constants are read from the supplied versioned registry. The
    /// charge/probe pair count is checked before pair iteration or contribution
    /// allocation. Any singularity or numeric failure returns no snapshot and
    /// cannot mutate canonical world or constants state.
    pub fn evaluate(
        &self,
        constants: &PhysicalConstants,
    ) -> Result<ElectrostaticSnapshot, ElectrostaticError> {
        let interactions = self
            .charges
            .len()
            .checked_mul(self.probes.len())
            .ok_or(ElectrostaticError::InteractionCountOverflow)?;
        if interactions > MAX_ELECTROSTATIC_INTERACTIONS {
            return Err(ElectrostaticError::InteractionBudgetExceeded {
                interactions,
                maximum: MAX_ELECTROSTATIC_INTERACTIONS,
            });
        }

        let vacuum_permittivity = constants.vacuum_permittivity();
        let charges = self
            .charges
            .values()
            .map(|charge| PointChargeSnapshot {
                entity: charge.entity,
                position: charge.position,
                charge: charge.charge,
            })
            .collect();
        let mut probes = Vec::with_capacity(self.probes.len());
        for (probe_id, position) in &self.probes {
            probes.push(self.evaluate_probe(*probe_id, *position, vacuum_permittivity.get())?);
        }

        Ok(ElectrostaticSnapshot {
            revision: self.revision,
            constants_version: constants.version(),
            constants_source: constants.source(),
            vacuum_permittivity,
            charges,
            probes,
        })
    }

    fn evaluate_probe(
        &self,
        probe: FieldProbeId,
        position: Position2,
        vacuum_permittivity: f64,
    ) -> Result<FieldProbeSnapshot, ElectrostaticError> {
        let mut x_contributions = Vec::with_capacity(self.charges.len());
        let mut y_contributions = Vec::with_capacity(self.charges.len());
        for charge in self.charges.values() {
            let (scaled_dx, scaled_dy, separation_scale) =
                scaled_separation(position, charge.position);
            if scaled_dx == 0.0 && scaled_dy == 0.0 {
                return Err(ElectrostaticError::SingularProbe {
                    probe,
                    charge: charge.entity,
                });
            }
            let coordinate_scale = scaled_dx.abs().max(scaled_dy.abs());
            let normalized_x = scaled_dx / coordinate_scale;
            let normalized_y = scaled_dy / coordinate_scale;
            let normalized_distance = normalized_x.hypot(normalized_y);
            let scale = scaled_field_value(
                &[charge.charge.get()],
                &[
                    4.0 * std::f64::consts::PI,
                    vacuum_permittivity,
                    separation_scale,
                    separation_scale,
                    coordinate_scale,
                    coordinate_scale,
                    normalized_distance,
                    normalized_distance,
                ],
                ElectrostaticArithmetic::FieldScale,
                Some(probe),
                Some(charge.entity),
            )?;
            x_contributions.push(scaled_field_value(
                &[scale, scaled_dx],
                &[coordinate_scale, normalized_distance],
                ElectrostaticArithmetic::FieldComponent,
                Some(probe),
                Some(charge.entity),
            )?);
            y_contributions.push(scaled_field_value(
                &[scale, scaled_dy],
                &[coordinate_scale, normalized_distance],
                ElectrostaticArithmetic::FieldComponent,
                Some(probe),
                Some(charge.entity),
            )?);
        }
        let x = canonical_sum(x_contributions, probe)?;
        let y = canonical_sum(y_contributions, probe)?;
        let magnitude = x.hypot(y);
        if !magnitude.is_finite() {
            return Err(ElectrostaticError::DerivedNonFinite {
                operation: ElectrostaticArithmetic::FieldMagnitude,
                probe: Some(probe),
                charge: None,
            });
        }
        Ok(FieldProbeSnapshot {
            probe,
            position,
            field: ElectricField2::new(NewtonsPerCoulomb::new(x)?, NewtonsPerCoulomb::new(y)?),
            magnitude: NewtonsPerCoulomb::new(magnitude)?,
        })
    }
}

fn scaled_separation(probe: Position2, source: Position2) -> (f64, f64, f64) {
    let direct_x = probe.x().get() - source.x().get();
    let direct_y = probe.y().get() - source.y().get();
    if direct_x.is_finite() && direct_y.is_finite() {
        return (direct_x, direct_y, 1.0);
    }

    // A finite-coordinate subtraction can overflow only when opposite signs
    // add magnitudes, so this branch has no cancellation to preserve. Scaling
    // first keeps that extended separation finite without softening it.
    let scale = probe
        .x()
        .get()
        .abs()
        .max(probe.y().get().abs())
        .max(source.x().get().abs())
        .max(source.y().get().abs());
    (
        probe.x().get() / scale - source.x().get() / scale,
        probe.y().get() / scale - source.y().get() / scale,
        scale,
    )
}

fn canonical_sum(values: Vec<f64>, probe: FieldProbeId) -> Result<f64, ElectrostaticError> {
    reproducible_sum(&values).map_err(|_| ElectrostaticError::DerivedNonFinite {
        operation: ElectrostaticArithmetic::FieldReduction,
        probe: Some(probe),
        charge: None,
    })
}

fn scaled_field_value(
    numerators: &[f64],
    denominators: &[f64],
    operation: ElectrostaticArithmetic,
    probe: Option<FieldProbeId>,
    charge: Option<EntityId>,
) -> Result<f64, ElectrostaticError> {
    match scaled_product_quotient(numerators, denominators) {
        Ok(value) => Ok(value),
        Err(ScaledProductError::Underflow) => Err(ElectrostaticError::PrecisionLoss {
            operation,
            probe,
            charge,
        }),
        Err(_) => Err(ElectrostaticError::DerivedNonFinite {
            operation,
            probe,
            charge,
        }),
    }
}
