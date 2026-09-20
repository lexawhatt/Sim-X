use std::{
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

use sim_logic::prelude::*;

use crate::{
    MenuAction,
    menu::state::{MenuState, SocialLink},
};

#[derive(Default)]
struct LinkOpener {
    child: Option<Child>,
    last_launch: Option<Instant>,
}

/// Installs native URL dispatch after the menu systems.
///
/// Only the three fixed product URLs can be opened. Requests come from confirmed
/// menu actions, are limited to one child and one launch per second, and never
/// wait on a browser in the frame loop. Errors appear as menu status. Headless
/// tests deliberately omit this service and inspect the requested intent.
pub fn install_link_opener(application: &mut Application<MenuAction>) -> LogicResult {
    application.register_app_resource(LinkOpener::default())?;
    application.add_frame_system(dispatch);
    Ok(())
}

fn dispatch(mut opener: AppResMut<LinkOpener>, mut state: Option<ResMut<MenuState>>) {
    if let Some(mut child) = opener.child.take() {
        match child.try_wait() {
            Ok(Some(status)) => {
                if !status.success() {
                    eprintln!("Browser launcher exited unsuccessfully: {status}");
                }
                if let Some(state) = state.as_mut() {
                    state.link_status = if status.success() {
                        "Link sent to browser."
                    } else {
                        "Browser could not open this link."
                    };
                }
            }
            Ok(None) => opener.child = Some(child),
            Err(error) => {
                eprintln!("Cannot check browser launcher: {error}");
                opener.child = Some(child);
                if let Some(state) = state.as_mut() {
                    state.link_status = "Browser status is unavailable.";
                }
            }
        }
    }
    let Some(mut state) = state else {
        return;
    };
    let Some(link) = state.pending_link.take() else {
        return;
    };
    if opener.child.is_some() {
        state.link_status = "Browser is still opening...";
        return;
    }
    let now = Instant::now();
    if opener
        .last_launch
        .is_some_and(|last| now.duration_since(last) < Duration::from_secs(1))
    {
        state.link_status = "Please wait before opening again.";
        return;
    }
    opener.last_launch = Some(now);
    match launch(link) {
        Ok(child) => {
            opener.child = Some(child);
            state.link_status = "Opening your browser...";
        }
        Err(error) => {
            eprintln!("Cannot open {}: {error}", link.url());
            state.link_status = "Browser could not open this link.";
        }
    }
}

fn launch(link: SocialLink) -> std::io::Result<Child> {
    #[cfg(target_os = "windows")]
    let mut command = {
        let mut command = Command::new("rundll32.exe");
        command.arg("url.dll,FileProtocolHandler");
        command
    };
    #[cfg(target_os = "macos")]
    let mut command = Command::new("open");
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let mut command = Command::new("xdg-open");
    command
        .arg(link.url())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
}
