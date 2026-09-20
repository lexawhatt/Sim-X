use sim_physics::*;

pub fn id(value: u64) -> BodyId {
    BodyId::new(value).unwrap()
}
pub fn link_id(value: u64) -> LinkId {
    LinkId::new(value).unwrap()
}
pub fn body(value: u64, x: f64, y: f64, mass: f64) -> BodyDesc {
    BodyDesc {
        id: id(value),
        position_m: Vec2::new(x, y),
        velocity_m_s: Vec2::ZERO,
        mass_kg: mass,
        mobility: Mobility::Dynamic,
    }
}
pub fn fixed(value: u64, x: f64, y: f64) -> BodyDesc {
    BodyDesc {
        mobility: Mobility::Fixed,
        ..body(value, x, y, 1.0)
    }
}
pub fn no_gravity() -> PhysicsSettings {
    PhysicsSettings {
        gravity_m_s2: Vec2::ZERO,
        ..PhysicsSettings::default()
    }
}
pub fn force(target: u64, source: u64, x: f64, y: f64) -> ForceInput {
    ForceInput {
        target: id(target),
        source: ForceSourceId::new(source).unwrap(),
        force_n: Vec2::new(x, y),
    }
}
pub fn close(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "actual={actual:e}; expected={expected:e}; tolerance={tolerance:e}"
    );
}
pub fn rod(number: u64, a: u64, b: u64, length: f64) -> LinkDesc {
    LinkDesc::Rod {
        id: link_id(number),
        a: id(a),
        b: id(b),
        length_m: length,
    }
}
