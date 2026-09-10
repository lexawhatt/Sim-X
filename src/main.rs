//! Native desktop entry point for Sim;X.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    sim_x::app::run().map_err(std::io::Error::other)?;
    Ok(())
}
