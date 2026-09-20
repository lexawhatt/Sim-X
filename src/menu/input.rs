use sim_logic::prelude::*;

use super::{
    layout,
    state::{Domain, MenuState, Overlay, PhysicsScale, Target},
};

/// Physical actions used by the single main-menu input owner.
pub use crate::actions::AppAction as MenuAction;

pub(crate) fn bind(app: &mut Application<MenuAction>) -> LogicResult {
    crate::actions::bind(app)
}

fn activate(state: &mut MenuState, target: Target, commands: &mut Commands) -> LogicResult {
    match target {
        Target::Domains => state.show(Overlay::Domains)?,
        Target::Settings => state.show(Overlay::Settings)?,
        Target::Quit => state.show(Overlay::Quit)?,
        Target::Close | Target::BackToMenu => state.show(Overlay::None)?,
        Target::ConfirmQuit => {
            commands.request_exit()?;
            state.departing = true;
            state.pointer.cancel();
        }
        Target::ReduceMotion => state.reduced_motion = !state.reduced_motion,
        Target::Domain(domain) => {
            if domain == Domain::Phys {
                state.show(Overlay::PhysicsScales)?;
            } else if domain == Domain::Math {
                state.pending_math = true;
                state.departing = true;
                state.pointer.cancel();
            } else {
                state.selected_domain = Some(domain);
                state.set_focus(Some(target))?;
            }
        }
        Target::Scale(PhysicsScale::Macro) => state.show(Overlay::Projects)?,
        Target::Scale(_) => {
            state.set_focus(Some(target))?;
            state.project_error = "Coming soon. Macro is available now.";
        }
        Target::BackToDomains => state.show(Overlay::Domains)?,
        Target::BackToScales => state.show(Overlay::PhysicsScales)?,
        Target::CreateProject => {
            state.project_name.clear();
            state.project_error = "";
            state.show(Overlay::CreateProject)?;
        }
        Target::ProjectName => state.set_focus(Some(Target::ProjectName))?,
        Target::ConfirmProject => {
            state.confirm_project();
        }
        Target::CancelProject => state.show(Overlay::Projects)?,
        Target::Social(link) => {
            // At most one external request per frame; the native adapter rate-limits it.
            state.pending_link = Some(link);
        }
    }
    Ok(())
}

pub(crate) fn route(
    input: FrameInput<MenuAction>,
    time: FrameTime,
    viewport: FrameViewport,
    state: Option<ResMut<MenuState>>,
    mut window: AppResMut<WindowControls>,
    mut commands: Commands,
) -> LogicResult {
    let Some(mut state) = state else {
        return Ok(());
    };
    // Menu input is frame-owned. Do not accumulate unused fixed-step edges.
    commands.set_paused(true)?;
    let mut navigation_anchor = state.last_pointer;
    let mut context_changed =
        input.focus_lost() || !layout::Layout::new(viewport.logical()).usable();
    if context_changed {
        state.pointer.cancel();
        state.focus.clear();
    } else {
        let scope = state.overlay;
        let targets = state.targets();
        state.focus.synchronize(scope, targets)?;
    }
    for edge in input.edges() {
        if matches!(
            edge.control(),
            InputControl::Key(PhysicalKeyCode::ControlLeft | PhysicalKeyCode::ControlRight)
        ) {
            let index =
                usize::from(edge.control() == InputControl::Key(PhysicalKeyCode::ControlRight));
            state.controls[index] = edge.state() == ButtonState::Pressed && !edge.is_cancelled();
        }
        if matches!(
            edge.control(),
            InputControl::Key(PhysicalKeyCode::ShiftLeft | PhysicalKeyCode::ShiftRight)
        ) {
            let index =
                usize::from(edge.control() == InputControl::Key(PhysicalKeyCode::ShiftRight));
            state.shifts[index] = edge.state() == ButtonState::Pressed && !edge.is_cancelled();
        }
        if edge.control() == InputControl::Key(PhysicalKeyCode::F11)
            && edge.state() == ButtonState::Pressed
            && !edge.is_cancelled()
        {
            state.fullscreen_requested = !state.fullscreen_requested;
            window.request_mode(if state.fullscreen_requested {
                WindowMode::BorderlessFullscreen(FullscreenMonitor::Automatic)
            } else {
                WindowMode::Windowed
            });
        }
        let hit = if state.departing || context_changed {
            None
        } else {
            layout::hit(&state, edge.pointer())
        };
        let outcome = state.pointer.process(edge, hit);
        if state.departing || context_changed {
            continue;
        }
        let previous_overlay = state.overlay;
        if let Some(PointerButtonEvent::Clicked { target, .. }) = outcome.event() {
            state.pointer_navigation = true;
            state.focus.clear();
            activate(&mut state, target, &mut commands)?;
        }
        if matches!(edge.control(), InputControl::Key(_))
            && edge.state() == ButtonState::Pressed
            && !edge.is_cancelled()
        {
            state.pointer_navigation = false;
            // Logic does not expose motion ordering around keyboard edges.
            // Conservatively give keyboard focus this frame's pointer sample.
            navigation_anchor = input.pointer();
            let InputControl::Key(key) = edge.control() else {
                continue;
            };
            let text_owned = state.overlay == Overlay::CreateProject
                && state.focus.focused() == Some(Target::ProjectName);
            if text_owned && key == PhysicalKeyCode::Backspace {
                state.project_name.pop();
                state.project_error = "";
            } else if text_owned && key == PhysicalKeyCode::Enter {
                state.confirm_project();
            } else if text_owned
                && !state.controls.iter().any(|held| *held)
                && let Some(character) =
                    project_character(key, state.shifts.iter().any(|held| *held))
            {
                state.append_project_character(character);
            } else {
                match key {
                    PhysicalKeyCode::ArrowUp
                    | PhysicalKeyCode::ArrowLeft
                    | PhysicalKeyCode::ArrowDown
                    | PhysicalKeyCode::ArrowRight
                    | PhysicalKeyCode::Tab => {
                        let backwards =
                            matches!(key, PhysicalKeyCode::ArrowUp | PhysicalKeyCode::ArrowLeft)
                                || (key == PhysicalKeyCode::Tab
                                    && state.shifts.iter().any(|held| *held));
                        state.navigate(if backwards {
                            FocusCommand::Previous
                        } else {
                            FocusCommand::Next
                        })?;
                    }
                    PhysicalKeyCode::Enter | PhysicalKeyCode::Space => {
                        if let Some(target) = state.navigate(FocusCommand::Activate)? {
                            activate(&mut state, target, &mut commands)?;
                        }
                    }
                    PhysicalKeyCode::Escape => {
                        let overlay = match state.overlay {
                            Overlay::None => Overlay::Quit,
                            Overlay::PhysicsScales => Overlay::Domains,
                            Overlay::Projects => Overlay::PhysicsScales,
                            Overlay::CreateProject => Overlay::Projects,
                            _ => Overlay::None,
                        };
                        state.show(overlay)?;
                    }
                    _ => {}
                }
            }
        }
        // A newly exposed layer cannot receive the tail of an old input batch.
        // Still feed suppressed edges above so a release never leaks next frame.
        context_changed = state.overlay != previous_overlay;
        if context_changed {
            state.pointer.cancel();
            navigation_anchor = if matches!(edge.control(), InputControl::Key(_)) {
                input.pointer()
            } else {
                edge.pointer()
            };
        }
    }
    // Movement after a mouse-driven screen transition can restore hover within
    // this batch; a stationary pointer must not reveal the new screen's motif.
    if input.pointer() != navigation_anchor {
        state.pointer_navigation = true;
    }
    if input.focus_lost() {
        state.pointer.cancel();
        state.focus.clear();
        state.shifts.fill(false);
        state.controls.fill(false);
    }
    state.last_pointer = input.pointer();
    // Hover uses today's layout; queued button edges above retain event-time
    // geometry. A resize alone must not highlight a button at its old location.
    let current_pointer = input
        .pointer()
        .map(|sample| PointerSample::new(sample.position(), viewport.logical()))
        .transpose()?;
    let hovered = if input.focus_lost() {
        None
    } else {
        layout::hit(&state, current_pointer)
    };
    state.hovered = if state.pointer_navigation && layout::Layout::new(viewport.logical()).usable()
    {
        hovered
    } else {
        None
    };
    window.request_cursor(
        if !input.focus_lost()
            && layout::Layout::new(viewport.logical()).usable()
            && hovered.is_some()
        {
            if hovered == Some(Target::ProjectName) {
                CursorShape::Text
            } else {
                CursorShape::Pointer
            }
        } else {
            CursorShape::Default
        },
    );
    let seconds = time.seconds_f32().clamp(0.0, 0.25);
    let preview_target = if state.overlay == Overlay::Domains {
        match state.hovered.or(state.focus.focused()) {
            Some(super::state::Target::Domain(domain)) => Some(domain),
            _ => None,
        }
    } else {
        None
    };
    state.preview_pulse = if state.reduced_motion || preview_target.is_none() {
        0.0
    } else if preview_target != state.last_preview_target {
        1.0
    } else {
        (state.preview_pulse - seconds / 0.5).max(0.0)
    };
    state.last_preview_target = preview_target;
    let blend = if state.reduced_motion {
        1.0
    } else {
        1.0 - (-16.0 * seconds).exp()
    };
    for target in Target::ALL {
        let on = state.targets().contains(&target)
            && (state.hovered == Some(target) || state.focus.focused() == Some(target));
        let desired = if on { 1.0 } else { 0.0 };
        state.emphasis[target.index()] += (desired - state.emphasis[target.index()]) * blend;
    }
    if !state.reduced_motion {
        state.phase = (state.phase + seconds * 0.6).rem_euclid(std::f32::consts::TAU * 10.0);
        if !state.exploration() {
            state.home_seconds = (state.home_seconds + seconds).rem_euclid(20.0);
            state.home_mix = home_mix(state.home_seconds);
        }
    }
    let transition = if state.reduced_motion {
        1.0
    } else {
        1.0 - (-6.0 * seconds).exp()
    };
    let target = if state.exploration() { 1.0 } else { 0.0 };
    state.domains_blend += (target - state.domains_blend) * transition;
    let physics = matches!(
        state.overlay,
        Overlay::PhysicsScales | Overlay::Projects | Overlay::CreateProject
    );
    state.scale_blend += (f32::from(physics) - state.scale_blend) * transition;
    for (index, overlay) in [
        Overlay::Domains,
        Overlay::PhysicsScales,
        Overlay::Projects,
        Overlay::CreateProject,
    ]
    .into_iter()
    .enumerate()
    {
        state.picker_mix[index] +=
            (f32::from(state.overlay == overlay) - state.picker_mix[index]) * transition;
    }
    let scale_preview = state.preview_scale();
    if !state.reduced_motion
        && state.overlay == Overlay::PhysicsScales
        && scale_preview == Some(PhysicsScale::Macro)
    {
        state.macro_seconds = (state.macro_seconds + seconds).rem_euclid(18.0);
    }
    for scale in PhysicsScale::ALL {
        state.scale_mix[scale.index()] +=
            (f32::from(scale_preview == Some(scale)) - state.scale_mix[scale.index()]) * transition;
    }
    let preview = state.preview_domain();
    for domain in super::state::Domain::ALL {
        let target = if Some(domain) == preview { 1.0 } else { 0.0 };
        state.domain_mix[domain.index()] +=
            (target - state.domain_mix[domain.index()]) * transition;
    }
    Ok(())
}

// Logic currently delivers physical keys rather than committed text/IME input.
// Keep that limitation visible in the form; do not guess the keyboard layout.
fn project_character(key: PhysicalKeyCode, shift: bool) -> Option<char> {
    use PhysicalKeyCode::*;
    let letter = match key {
        KeyA => 'a',
        KeyB => 'b',
        KeyC => 'c',
        KeyD => 'd',
        KeyE => 'e',
        KeyF => 'f',
        KeyG => 'g',
        KeyH => 'h',
        KeyI => 'i',
        KeyJ => 'j',
        KeyK => 'k',
        KeyL => 'l',
        KeyM => 'm',
        KeyN => 'n',
        KeyO => 'o',
        KeyP => 'p',
        KeyQ => 'q',
        KeyR => 'r',
        KeyS => 's',
        KeyT => 't',
        KeyU => 'u',
        KeyV => 'v',
        KeyW => 'w',
        KeyX => 'x',
        KeyY => 'y',
        KeyZ => 'z',
        Digit0 => return Some('0'),
        Digit1 => return Some('1'),
        Digit2 => return Some('2'),
        Digit3 => return Some('3'),
        Digit4 => return Some('4'),
        Digit5 => return Some('5'),
        Digit6 => return Some('6'),
        Digit7 => return Some('7'),
        Digit8 => return Some('8'),
        Digit9 => return Some('9'),
        Space => return Some(' '),
        Minus => return Some(if shift { '_' } else { '-' }),
        _ => return None,
    };
    Some(if shift {
        letter.to_ascii_uppercase()
    } else {
        letter
    })
}

// Five seconds per subject, with a one-second smoothstep crossfade at the end.
// The analytical weights stay normalized and never expose more than two motifs.
pub(crate) fn home_mix(seconds: f32) -> [f32; 4] {
    let cycle = seconds.rem_euclid(20.0);
    let index = (cycle / 5.0) as usize;
    let fraction = (cycle.rem_euclid(5.0) - 4.0).clamp(0.0, 1.0);
    let fade = fraction * fraction * (3.0 - 2.0 * fraction);
    let mut weights = [0.0; 4];
    weights[index] = 1.0 - fade;
    weights[(index + 1) % 4] = fade;
    weights
}
