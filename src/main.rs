//! Native entry point. Sim;Logic owns the window, input, and rendering lifecycle.

fn main() -> sim_logic::LogicResult {
    let editor = match parse_arguments(std::env::args().skip(1))? {
        Some(editor) => editor,
        None => return Ok(()),
    };
    let title = match editor {
        Launch::Physics => "Sim;X - Physics Editor",
        Launch::Math => "Sim;X - Mathematics",
        Launch::Menu => "Sim;X",
    };
    let mut desktop = sim_logic::prelude::DesktopConfig::new(title, 1280.0, 800.0)?;
    desktop.set_window_mode(sim_logic::prelude::WindowMode::BorderlessFullscreen(
        sim_logic::prelude::FullscreenMonitor::Automatic,
    ));
    if editor == Launch::Physics {
        let (application, initial) = sim_x::build_phys_editor_application()?;
        application.run_desktop(initial, desktop)?;
    } else if editor == Launch::Math {
        let (mut application, initial) = sim_x::build_math_editor_application()?;
        sim_x::platform::install_clipboard(&mut application)?;
        application.run_desktop(initial, desktop)?;
    } else {
        let (mut application, initial) = sim_x::build_application()?;
        sim_x::platform::install_link_opener(&mut application)?;
        sim_x::platform::install_clipboard(&mut application)?;
        application.run_desktop(initial, desktop)?;
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Launch {
    Menu,
    Physics,
    Math,
}
fn parse_arguments(arguments: impl IntoIterator<Item = String>) -> Result<Option<Launch>, String> {
    let mut editor = Launch::Menu;
    for argument in arguments {
        match argument.as_str() {
            "--phys_editor" | "--phys-editor" => editor = Launch::Physics,
            "--math_editor" | "--math-editor" => editor = Launch::Math,
            "--help" | "-h" => {
                println!(
                    "Sim;X\n  cargo run                     Main menu\n  cargo run -- --phys_editor     Physics editor\n  cargo run -- --math_editor     Euclidean Math workspace\n\nDocuments are in memory only. Math: structured formula input, functions, numerical integrals, linked points and shapes. Physics Run starts separate mechanics."
                );
                return Ok(None);
            }
            _ => {
                return Err(format!("unknown argument {argument:?}; use --help"));
            }
        }
    }
    Ok(Some(editor))
}

#[cfg(test)]
mod tests {
    use super::{Launch, parse_arguments};
    #[test]
    fn launch_mode_is_explicit_and_unknown_options_are_errors() {
        assert_eq!(parse_arguments([]), Ok(Some(Launch::Menu)));
        for flag in ["--phys_editor", "--phys-editor"] {
            assert_eq!(
                parse_arguments([flag.to_owned()]),
                Ok(Some(Launch::Physics))
            );
        }
        assert!(parse_arguments(["--phys_edtor".to_owned()]).is_err());
        assert_eq!(
            parse_arguments(["--math_editor".to_owned()]),
            Ok(Some(Launch::Math))
        );
        assert_eq!(parse_arguments(["--help".to_owned()]), Ok(None));
    }
}
