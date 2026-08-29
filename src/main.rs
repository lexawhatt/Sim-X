mod app;
mod presentation;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    app::run().map_err(std::io::Error::other)?;
    Ok(())
}
