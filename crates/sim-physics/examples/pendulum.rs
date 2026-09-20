//! Headless reference graph: `cargo run -p sim-physics --example pendulum`.

use sim_physics::{PhysicsSettings, scenarios};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let scene = scenarios::pendulum(1.0, 2.0, 0.3, PhysicsSettings::default())?;
    let mut world = scene.build()?;
    for _ in 0..2400 {
        let report = world.step(&[])?;
        if report.step_index % 240 == 0 {
            println!(
                "t={:.3}s E={:.8}J rod_error={:.3e}m reaction_A=({:.4},{:.4})N",
                report.elapsed_s,
                report.energy.total_j,
                report.links[0].extension_m,
                report.links[0].force_on_a_n.x,
                report.links[0].force_on_a_n.y
            );
        }
    }
    Ok(())
}
