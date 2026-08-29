/// Starts the Sim;X application shell.
pub(crate) fn run() -> Result<(), String> {
    crate::presentation::ui::run().map_err(|error| error.to_string())
}
