//! Explicit user-requested system clipboard IO, isolated from the UI frame and
//! scientific state. Keep the backend alive for X11/Wayland clipboard ownership.
use crate::{
    actions::AppAction,
    math_editor::{clipboard::ClipboardAction, state::MathState},
};
use sim_logic::prelude::*;
use std::sync::{
    Mutex,
    mpsc::{self, Receiver, SyncSender},
};

#[derive(Resource)]
struct ClipboardService {
    commands: SyncSender<Option<String>>,
    results: Mutex<Receiver<Result<String, String>>>,
    active: Option<std::sync::Arc<()>>,
}
/// Installs asynchronous native text clipboard handling for Math. Headless
/// builders do not install this service and never access the system clipboard.
pub fn install_clipboard(application: &mut Application<AppAction>) -> LogicResult {
    let (send, receive) = mpsc::sync_channel::<Option<String>>(1);
    let (results, replies) = mpsc::sync_channel(1);
    std::thread::Builder::new()
        .name("sim-clipboard".into())
        .spawn(move || {
            let mut clipboard = None;
            while let Ok(request) = receive.recv() {
                let result = (|| {
                    if clipboard.is_none() {
                        clipboard = Some(arboard::Clipboard::new().map_err(|e| e.to_string())?);
                    }
                    let Some(backend) = clipboard.as_mut() else {
                        return Err("Clipboard unavailable".into());
                    };
                    if let Some(text) = request {
                        backend.set_text(text).map_err(|e| e.to_string())?;
                        Ok(String::new())
                    } else {
                        backend.get_text().map_err(|e| e.to_string())
                    }
                })();
                if results.send(result).is_err() {
                    break;
                }
            }
        })
        .map_err(|e| format!("Cannot start clipboard service: {e}"))?;
    application.register_app_resource(ClipboardService {
        commands: send,
        results: Mutex::new(replies),
        active: None,
    })?;
    application.add_frame_system(dispatch);
    Ok(())
}
fn dispatch(mut service: AppResMut<ClipboardService>, mut state: Option<ResMut<MathState>>) {
    let response = service
        .results
        .lock()
        .ok()
        .and_then(|receiver| receiver.try_recv().ok());
    if let Some(response) = response {
        let active = service.active.take();
        if let Some(state) = state.as_mut()
            && let (Some(active), Some(request)) = (active, state.clipboard_pending.as_ref())
            && std::sync::Arc::ptr_eq(&active, &request.token)
        {
            state.complete_clipboard(response);
        }
    }
    let Some(mut state) = state else {
        return;
    };
    if service.active.is_some() {
        return;
    }
    if let Some(request) = &state.clipboard_pending {
        let payload = (request.action != ClipboardAction::Paste).then(|| request.text.clone());
        match service.commands.try_send(payload) {
            Ok(()) => service.active = Some(request.token.clone()),
            Err(error) => state.complete_clipboard(Err(error.to_string())),
        }
    }
}
