use std::collections::{BTreeMap, BTreeSet};

use crate::foundation::{
    EntityId, Seconds, StepIndex, TimeStep, WorldRevision,
    reproducible_sum::{ReproducibleSumError, reproducible_sum},
};

use super::{
    ConductiveLinkSpec, Joules, Kelvin, ThermalArithmetic, ThermalBodySnapshot, ThermalBodySpec,
    ThermalEnergyTotal, ThermalError, ThermalEvent, ThermalEventKind, ThermalLinkId,
    ThermalLinkSnapshot, ThermalSnapshot, ThermalStepReport, Watts,
    model::{ConductiveLink, ThermalBody},
};

/// Hard maximum number of canonical thermal bodies in one world.
pub const MAX_THERMAL_BODIES: usize = 1_024;
/// Hard maximum number of conductive links in one world.
pub const MAX_THERMAL_LINKS: usize = 4_096;
/// Maximum typed event count produced by one successful step.
pub const MAX_THERMAL_EVENTS: usize = MAX_THERMAL_LINKS + MAX_THERMAL_BODIES + 1;

/// Canonical bounded world for lumped conductive heat transfer.
///
/// All state uses SI `f64` quantities. Structural changes and fixed steps
/// validate complete candidate state before commit. The world has no renderer,
/// wall-clock, or presentation dependency.
#[derive(Clone, Debug, PartialEq)]
pub struct ThermalWorld {
    bodies: BTreeMap<EntityId, ThermalBody>,
    links: BTreeMap<ThermalLinkId, ConductiveLink>,
    link_by_endpoints: BTreeMap<(EntityId, EntityId), ThermalLinkId>,
    next_entity_id: Option<u64>,
    next_link_id: Option<u64>,
    step: StepIndex,
    revision: WorldRevision,
    elapsed: Seconds,
}

impl Default for ThermalWorld {
    fn default() -> Self {
        Self::new()
    }
}

impl ThermalWorld {
    /// Creates an empty thermal world at step, revision, and elapsed time zero.
    pub fn new() -> Self {
        Self {
            bodies: BTreeMap::new(),
            links: BTreeMap::new(),
            link_by_endpoints: BTreeMap::new(),
            next_entity_id: Some(1),
            next_link_id: Some(1),
            step: StepIndex::ZERO,
            revision: WorldRevision::ZERO,
            elapsed: Seconds::ZERO,
        }
    }

    /// Creates one internally isothermal body from kelvin and J/K values.
    ///
    /// Initial thermal energy is derived as `temperature * heat_capacity`.
    /// Overflow, underflow, capacity, identity, or revision failure leaves the
    /// complete world unchanged.
    pub fn create_body(&mut self, spec: ThermalBodySpec) -> Result<EntityId, ThermalError> {
        if self.bodies.len() >= MAX_THERMAL_BODIES {
            return Err(ThermalError::BodyCapacityReached {
                maximum: MAX_THERMAL_BODIES,
            });
        }
        let raw_id = self
            .next_entity_id
            .ok_or(ThermalError::EntityIdentityExhausted)?;
        let next_revision = self
            .revision
            .checked_next()
            .ok_or(ThermalError::CounterExhausted)?;
        let energy = checked_product(
            spec.temperature.get(),
            spec.heat_capacity.get(),
            ThermalArithmetic::InitialEnergy,
            None,
        )?;
        let energy = Joules::new(energy)?;
        let entity = EntityId::new(raw_id).map_err(|_| ThermalError::EntityIdentityExhausted)?;

        self.bodies.insert(
            entity,
            ThermalBody {
                energy_joules: energy.get(),
                heat_capacity: spec.heat_capacity,
                last_net_power: Watts::ZERO,
            },
        );
        self.next_entity_id = raw_id.checked_add(1);
        self.revision = next_revision;
        Ok(entity)
    }

    /// Creates one persistent conductive relationship between two bodies.
    ///
    /// Endpoint order is canonicalized. Missing, identical, duplicate, or
    /// over-capacity proposals commit nothing.
    pub fn create_link(&mut self, spec: ConductiveLinkSpec) -> Result<ThermalLinkId, ThermalError> {
        if self.links.len() >= MAX_THERMAL_LINKS {
            return Err(ThermalError::LinkCapacityReached {
                maximum: MAX_THERMAL_LINKS,
            });
        }
        let (first, second) = canonical_endpoints(spec.first, spec.second)?;
        for entity in [first, second] {
            if !self.bodies.contains_key(&entity) {
                return Err(ThermalError::MissingBody { entity });
            }
        }
        if let Some(existing) = self.link_by_endpoints.get(&(first, second)) {
            return Err(ThermalError::DuplicateLink {
                first,
                second,
                existing: *existing,
            });
        }
        let raw_id = self
            .next_link_id
            .ok_or(ThermalError::LinkIdentityExhausted)?;
        let next_revision = self
            .revision
            .checked_next()
            .ok_or(ThermalError::CounterExhausted)?;
        let link = ThermalLinkId::new(raw_id).ok_or(ThermalError::LinkIdentityExhausted)?;

        self.links.insert(
            link,
            ConductiveLink {
                first,
                second,
                conductance: spec.conductance,
                last_first_to_second_power: Watts::ZERO,
            },
        );
        self.link_by_endpoints.insert((first, second), link);
        self.next_link_id = raw_id.checked_add(1);
        self.revision = next_revision;
        Ok(link)
    }

    /// Advances the closed conduction model by one validated fixed step.
    ///
    /// The algorithm is simultaneous explicit Euler with a per-body stability
    /// ratio no greater than one, exact reproducible reductions, checked
    /// `f64` progress, and atomic commit.
    pub fn step(&mut self, dt: TimeStep) -> Result<ThermalStepReport, ThermalError> {
        let next_step = self
            .step
            .checked_next()
            .ok_or(ThermalError::CounterExhausted)?;
        let next_revision = self
            .revision
            .checked_next()
            .ok_or(ThermalError::CounterExhausted)?;
        let next_elapsed_value = checked_sum(
            self.elapsed.get(),
            dt.get(),
            ThermalArithmetic::ElapsedTime,
            None,
        )?;
        if next_elapsed_value == self.elapsed.get() {
            return Err(ThermalError::PrecisionLoss {
                operation: ThermalArithmetic::ElapsedTime,
                entity: None,
            });
        }
        let next_elapsed =
            Seconds::new(next_elapsed_value).map_err(|_| ThermalError::DerivedNonFinite {
                operation: ThermalArithmetic::ElapsedTime,
                entity: None,
            })?;

        self.validate_stability(dt)?;
        let mut requested_heats = BTreeMap::new();
        let mut candidate_links = self.links.clone();

        for (link_id, link) in &self.links {
            let first_temperature = self.temperature(link.first)?;
            let second_temperature = self.temperature(link.second)?;
            let difference = checked_sum(
                first_temperature,
                -second_temperature,
                ThermalArithmetic::TemperatureDifference,
                None,
            )?;
            let requested_power = checked_product(
                link.conductance.get(),
                difference,
                ThermalArithmetic::ConductivePower,
                None,
            )?;
            let requested_heat = checked_product(
                requested_power,
                dt.get(),
                ThermalArithmetic::TransferredHeat,
                None,
            )?;
            requested_heats.insert(*link_id, requested_heat);
            candidate_links
                .get_mut(link_id)
                .ok_or(ThermalError::LinkIdentityExhausted)?
                .last_first_to_second_power = Watts::new(requested_power)?;
        }

        let mut active_links: BTreeSet<_> = self.links.keys().copied().collect();
        let energy_deltas = loop {
            let energy_deltas =
                reduce_energy_deltas(&self.bodies, &self.links, &requested_heats, &active_links)?;
            let mut absorbed_bodies = BTreeSet::new();
            for (entity, body) in &self.bodies {
                let delta = *energy_deltas
                    .get(entity)
                    .ok_or(ThermalError::MissingBody { entity: *entity })?;
                if delta == 0.0 {
                    continue;
                }
                let next_energy = body.energy_joules + delta;
                if next_energy == body.energy_joules {
                    absorbed_bodies.insert(*entity);
                }
            }
            if absorbed_bodies.is_empty() {
                break energy_deltas;
            }

            let active_count = active_links.len();
            active_links.retain(|link_id| {
                let Some(link) = self.links.get(link_id) else {
                    return false;
                };
                !absorbed_bodies.contains(&link.first) && !absorbed_bodies.contains(&link.second)
            });
            debug_assert!(active_links.len() < active_count);
        };

        for (link_id, link) in &mut candidate_links {
            if !active_links.contains(link_id) {
                link.last_first_to_second_power = Watts::ZERO;
            }
        }

        let mut candidate_bodies = self.bodies.clone();
        for (entity, body) in &self.bodies {
            let delta = *energy_deltas
                .get(entity)
                .ok_or(ThermalError::MissingBody { entity: *entity })?;
            let next_energy = checked_sum(
                body.energy_joules,
                delta,
                ThermalArithmetic::EnergyUpdate,
                Some(*entity),
            )?;
            if next_energy <= 0.0 {
                return Err(ThermalError::NonPositiveCandidateEnergy { entity: *entity });
            }
            let net_power =
                checked_quotient(delta, dt.get(), ThermalArithmetic::NetPower, Some(*entity))?;
            let candidate = candidate_bodies
                .get_mut(entity)
                .ok_or(ThermalError::MissingBody { entity: *entity })?;
            candidate.energy_joules = next_energy;
            candidate.last_net_power = Watts::new(net_power)?;
        }

        let mut energy_drift_terms = Vec::with_capacity(candidate_bodies.len() * 2);
        for (entity, candidate) in &candidate_bodies {
            let committed = self
                .bodies
                .get(entity)
                .ok_or(ThermalError::MissingBody { entity: *entity })?;
            energy_drift_terms.extend([candidate.energy_joules, -committed.energy_joules]);
        }
        let drift = canonical_sum(
            energy_drift_terms,
            ThermalArithmetic::EnergyAccounting,
            None,
        )?
        .abs();
        let tolerance_factor = 8.0 * f64::EPSILON;
        let tolerance = canonical_sum(
            self.bodies
                .values()
                .map(|body| tolerance_factor * body.energy_joules.abs()),
            ThermalArithmetic::EnergyAccounting,
            None,
        )?;
        if drift > tolerance {
            return Err(ThermalError::EnergyConservationLost {
                drift_joules: drift,
                tolerance_joules: tolerance,
            });
        }

        self.bodies = candidate_bodies;
        self.links = candidate_links;
        self.step = next_step;
        self.revision = next_revision;
        self.elapsed = next_elapsed;

        let snapshot = self.snapshot()?;
        let events = self.events_from_snapshot(&snapshot);
        debug_assert!(events.len() <= MAX_THERMAL_EVENTS);
        Ok(ThermalStepReport { events, snapshot })
    }

    /// Produces a bounded immutable snapshot without changing the world.
    pub fn snapshot(&self) -> Result<ThermalSnapshot, ThermalError> {
        let bodies = self
            .bodies
            .iter()
            .map(|(entity, body)| {
                Ok(ThermalBodySnapshot {
                    entity: *entity,
                    temperature: Kelvin::new(self.temperature(*entity)?)?,
                    energy: Joules::new(body.energy_joules)?,
                    heat_capacity: body.heat_capacity,
                    net_heat_power: body.last_net_power,
                })
            })
            .collect::<Result<Vec<_>, ThermalError>>()?;
        let total_energy = if bodies.is_empty() {
            ThermalEnergyTotal::Empty
        } else {
            let energies: Vec<_> = bodies.iter().map(|body| body.energy.get()).collect();
            match reproducible_sum(&energies) {
                Ok(total) => ThermalEnergyTotal::Representable(Joules::new(total)?),
                Err(ReproducibleSumError::NonFiniteResult) => {
                    ThermalEnergyTotal::AboveBinary64Range
                }
                Err(_) => {
                    return Err(ThermalError::DerivedNonFinite {
                        operation: ThermalArithmetic::EnergyAccounting,
                        entity: None,
                    });
                }
            }
        };
        let links = self
            .links
            .iter()
            .map(|(link_id, link)| ThermalLinkSnapshot {
                link: *link_id,
                first: link.first,
                second: link.second,
                conductance: link.conductance,
                first_to_second_power: link.last_first_to_second_power,
            })
            .collect();
        Ok(ThermalSnapshot {
            step: self.step,
            revision: self.revision,
            elapsed: self.elapsed,
            total_energy,
            bodies,
            links,
        })
    }

    fn temperature(&self, entity: EntityId) -> Result<f64, ThermalError> {
        let body = self
            .bodies
            .get(&entity)
            .ok_or(ThermalError::MissingBody { entity })?;
        checked_quotient(
            body.energy_joules,
            body.heat_capacity.get(),
            ThermalArithmetic::Temperature,
            Some(entity),
        )
    }

    fn validate_stability(&self, dt: TimeStep) -> Result<(), ThermalError> {
        for (link_id, link) in &self.links {
            let first_capacity = self
                .bodies
                .get(&link.first)
                .ok_or(ThermalError::MissingBody { entity: link.first })?
                .heat_capacity
                .get();
            let second_capacity = self
                .bodies
                .get(&link.second)
                .ok_or(ThermalError::MissingBody {
                    entity: link.second,
                })?
                .heat_capacity
                .get();
            let conductance_dt = checked_product(
                dt.get(),
                link.conductance.get(),
                ThermalArithmetic::StabilityRatio,
                None,
            )?;
            let ratio = canonical_stability_sum([
                checked_quotient(
                    conductance_dt,
                    first_capacity,
                    ThermalArithmetic::StabilityRatio,
                    Some(link.first),
                )?,
                checked_quotient(
                    conductance_dt,
                    second_capacity,
                    ThermalArithmetic::StabilityRatio,
                    Some(link.second),
                )?,
            ])?;
            if ratio > 1.0 {
                return Err(ThermalError::UnstableLinkTimeStep {
                    link: *link_id,
                    ratio,
                });
            }
        }

        let mut incident_ratios: BTreeMap<EntityId, Vec<f64>> = self
            .bodies
            .keys()
            .copied()
            .map(|entity| (entity, Vec::new()))
            .collect();
        for link in self.links.values() {
            let conductance_dt = checked_product(
                dt.get(),
                link.conductance.get(),
                ThermalArithmetic::StabilityRatio,
                None,
            )?;
            for entity in [link.first, link.second] {
                let capacity = self
                    .bodies
                    .get(&entity)
                    .ok_or(ThermalError::MissingBody { entity })?
                    .heat_capacity
                    .get();
                let ratio = checked_quotient(
                    conductance_dt,
                    capacity,
                    ThermalArithmetic::StabilityRatio,
                    Some(entity),
                )?;
                incident_ratios
                    .get_mut(&entity)
                    .ok_or(ThermalError::MissingBody { entity })?
                    .push(ratio);
            }
        }
        for (entity, ratios) in incident_ratios {
            let ratio = canonical_sum(ratios, ThermalArithmetic::StabilityRatio, Some(entity))?;
            if ratio > 1.0 {
                return Err(ThermalError::UnstableTimeStep { entity, ratio });
            }
        }
        Ok(())
    }

    fn events_from_snapshot(&self, snapshot: &ThermalSnapshot) -> Vec<ThermalEvent> {
        let mut events = Vec::with_capacity(snapshot.links.len() + snapshot.bodies.len() + 1);
        events.extend(snapshot.links.iter().map(|link| ThermalEvent {
            step: snapshot.step,
            revision: snapshot.revision,
            elapsed: snapshot.elapsed,
            kind: ThermalEventKind::HeatTransferred {
                link: link.link,
                first_to_second_power: link.first_to_second_power,
            },
        }));
        events.extend(snapshot.bodies.iter().map(|body| ThermalEvent {
            step: snapshot.step,
            revision: snapshot.revision,
            elapsed: snapshot.elapsed,
            kind: ThermalEventKind::BodyTemperatureChanged {
                entity: body.entity,
                temperature: body.temperature,
                net_heat_power: body.net_heat_power,
            },
        }));
        events.push(ThermalEvent {
            step: snapshot.step,
            revision: snapshot.revision,
            elapsed: snapshot.elapsed,
            kind: ThermalEventKind::StepCommitted {
                bodies: snapshot.bodies.len(),
                links: snapshot.links.len(),
            },
        });
        events
    }
}

fn canonical_endpoints(
    first: EntityId,
    second: EntityId,
) -> Result<(EntityId, EntityId), ThermalError> {
    if first == second {
        Err(ThermalError::IdenticalEndpoints { entity: first })
    } else if first < second {
        Ok((first, second))
    } else {
        Ok((second, first))
    }
}

fn reduce_energy_deltas(
    bodies: &BTreeMap<EntityId, ThermalBody>,
    links: &BTreeMap<ThermalLinkId, ConductiveLink>,
    requested_heats: &BTreeMap<ThermalLinkId, f64>,
    active_links: &BTreeSet<ThermalLinkId>,
) -> Result<BTreeMap<EntityId, f64>, ThermalError> {
    let mut contributions: BTreeMap<EntityId, Vec<f64>> = bodies
        .keys()
        .copied()
        .map(|entity| (entity, Vec::new()))
        .collect();
    for link_id in active_links {
        let link = links
            .get(link_id)
            .ok_or(ThermalError::LinkIdentityExhausted)?;
        let heat = *requested_heats
            .get(link_id)
            .ok_or(ThermalError::LinkIdentityExhausted)?;
        contributions
            .get_mut(&link.first)
            .ok_or(ThermalError::MissingBody { entity: link.first })?
            .push(-heat);
        contributions
            .get_mut(&link.second)
            .ok_or(ThermalError::MissingBody {
                entity: link.second,
            })?
            .push(heat);
    }

    contributions
        .into_iter()
        .map(|(entity, values)| {
            canonical_sum(values, ThermalArithmetic::EnergyReduction, Some(entity))
                .map(|delta| (entity, delta))
        })
        .collect()
}

fn canonical_sum(
    values: impl IntoIterator<Item = f64>,
    operation: ThermalArithmetic,
    entity: Option<EntityId>,
) -> Result<f64, ThermalError> {
    let values: Vec<f64> = values.into_iter().collect();
    reproducible_sum(&values).map_err(|_| ThermalError::DerivedNonFinite { operation, entity })
}

fn canonical_stability_sum(values: impl IntoIterator<Item = f64>) -> Result<f64, ThermalError> {
    canonical_sum(values, ThermalArithmetic::StabilityRatio, None)
}

fn checked_sum(
    left: f64,
    right: f64,
    operation: ThermalArithmetic,
    entity: Option<EntityId>,
) -> Result<f64, ThermalError> {
    let value = left + right;
    if value.is_finite() {
        Ok(if value == 0.0 { 0.0 } else { value })
    } else {
        Err(ThermalError::DerivedNonFinite { operation, entity })
    }
}

fn checked_product(
    left: f64,
    right: f64,
    operation: ThermalArithmetic,
    entity: Option<EntityId>,
) -> Result<f64, ThermalError> {
    let value = left * right;
    if !value.is_finite() {
        Err(ThermalError::DerivedNonFinite { operation, entity })
    } else if left != 0.0 && right != 0.0 && value == 0.0 {
        Err(ThermalError::PrecisionLoss { operation, entity })
    } else {
        Ok(if value == 0.0 { 0.0 } else { value })
    }
}

fn checked_quotient(
    numerator: f64,
    denominator: f64,
    operation: ThermalArithmetic,
    entity: Option<EntityId>,
) -> Result<f64, ThermalError> {
    let value = numerator / denominator;
    if !value.is_finite() {
        Err(ThermalError::DerivedNonFinite { operation, entity })
    } else if numerator != 0.0 && value == 0.0 {
        Err(ThermalError::PrecisionLoss { operation, entity })
    } else {
        Ok(if value == 0.0 { 0.0 } else { value })
    }
}

#[cfg(test)]
mod internal_tests {
    use super::*;
    use crate::foundation::TimeStep;

    fn body_spec() -> ThermalBodySpec {
        ThermalBodySpec::new(
            Kelvin::new(300.0).expect("temperature"),
            super::super::JoulesPerKelvin::new(10.0).expect("capacity"),
        )
    }

    fn conductance() -> super::super::WattsPerKelvin {
        super::super::WattsPerKelvin::new(1.0).expect("conductance")
    }

    #[test]
    fn exhausted_step_and_revision_counters_reject_atomically() {
        let mut exhausted_step = ThermalWorld::new();
        exhausted_step.step = StepIndex::new(u64::MAX);
        let before = exhausted_step.clone();
        assert_eq!(
            exhausted_step.step(TimeStep::new(1.0).expect("dt")),
            Err(ThermalError::CounterExhausted)
        );
        assert_eq!(exhausted_step, before);

        let mut exhausted_revision = ThermalWorld::new();
        exhausted_revision.revision = WorldRevision::new(u64::MAX);
        let before = exhausted_revision.clone();
        assert_eq!(
            exhausted_revision.step(TimeStep::new(1.0).expect("dt")),
            Err(ThermalError::CounterExhausted)
        );
        assert_eq!(exhausted_revision, before);

        let before = exhausted_revision.clone();
        assert_eq!(
            exhausted_revision.create_body(body_spec()),
            Err(ThermalError::CounterExhausted)
        );
        assert_eq!(exhausted_revision, before);
    }

    #[test]
    fn exhausted_body_and_link_id_allocators_reject_atomically() {
        let mut exhausted_body_ids = ThermalWorld::new();
        exhausted_body_ids.next_entity_id = None;
        let before = exhausted_body_ids.clone();
        assert_eq!(
            exhausted_body_ids.create_body(body_spec()),
            Err(ThermalError::EntityIdentityExhausted)
        );
        assert_eq!(exhausted_body_ids, before);

        let mut exhausted_link_ids = ThermalWorld::new();
        let first = exhausted_link_ids
            .create_body(body_spec())
            .expect("first body");
        let second = exhausted_link_ids
            .create_body(body_spec())
            .expect("second body");
        exhausted_link_ids.next_link_id = None;
        let before = exhausted_link_ids.clone();
        assert_eq!(
            exhausted_link_ids.create_link(ConductiveLinkSpec::new(first, second, conductance(),)),
            Err(ThermalError::LinkIdentityExhausted)
        );
        assert_eq!(exhausted_link_ids, before);

        exhausted_link_ids.next_link_id = Some(1);
        exhausted_link_ids.revision = WorldRevision::new(u64::MAX);
        let before = exhausted_link_ids.clone();
        assert_eq!(
            exhausted_link_ids.create_link(ConductiveLinkSpec::new(first, second, conductance(),)),
            Err(ThermalError::CounterExhausted)
        );
        assert_eq!(exhausted_link_ids, before);
    }
}
