use super::{
    document::{EditError, PhysicsEnvironment, Point},
    placement::Placement,
    session::{PlaybackSpeed, RunSession},
    state::{Camera, Control, EditorState, Mode, Tool},
};

pub(crate) fn report(state: &mut EditorState, result: Result<(), EditError>) {
    state.status = match result {
        Ok(()) => "Scene changed. In memory only - saving is not implemented yet.",
        Err(EditError::TooManyObjects) => {
            "Scene limit reached: 128 objects. Delete an object before adding more."
        }
        Err(EditError::IdExhausted) => "Object identifiers exhausted. No change was made.",
        Err(EditError::MissingObject(_)) => "That object no longer exists. No change was made.",
        Err(EditError::InvalidPosition) => {
            "Position must stay within +/-1,000,000 metres. No change was made."
        }
        Err(EditError::InvalidMass) => "Mass range: 0.001 to 1,000,000 kg. No change was made.",
        Err(EditError::InvalidSize) => "Size range: 0.1 to 1,000 metres. No change was made.",
        Err(EditError::InvalidRotation) => "Rotation must be finite. No change was made.",
        Err(EditError::InvalidRestitution) => "Restitution range: 0 to 1. No change was made.",
        Err(EditError::HeightRequiresBox) => "Only boxes have an independent height.",
        Err(EditError::AnchorMustRemainFixed) => "Anchors must stay fixed.",
        Err(EditError::AnchorHasNoCollider) => {
            "Anchors are attachment markers, not collision surfaces."
        }
        Err(EditError::AnchorMassIsFixed) => "An anchor's mass cannot be edited.",
        Err(EditError::TooManyLinks) => "Scene limit: 256 relationships. No change was made.",
        Err(EditError::LinkIdExhausted) => {
            "Relationship identifiers exhausted. No change was made."
        }
        Err(EditError::InvalidEnvironment) => "Unsupported environment range. No change was made.",
        Err(EditError::SelfLink) => "Choose two different bodies for a relationship.",
        Err(EditError::FixedEndpoints) => "A relationship needs at least one dynamic body.",
        Err(EditError::DuplicateLink) => "These bodies already have a relationship.",
        Err(EditError::InvalidLinkLength) => "Endpoints must remain separated; no change was made.",
        Err(EditError::InvalidAttachment) => {
            "Attachment must be inside its body. No change was made."
        }
    };
    if state
        .selected
        .is_some_and(|id| state.document.object(id).is_none())
    {
        state.select_one(None);
    }
}

pub(crate) fn activate(state: &mut EditorState, control: Control) {
    if !state.enabled(control) {
        return;
    }
    state.cancel_gesture();
    state.notice = None;
    match control {
        Control::Tool(tool) => {
            state.tool = tool;
            state.status = match tool {
                Tool::Spring => {
                    "Click two body surfaces for a spring. Corners snap; anchors use their centre."
                }
                Tool::Rod => "Click two bodies for a centre-to-centre rod. Escape cancels.",
                _ => "Tool changed. Drag with MMB to pan; scroll to zoom.",
            };
        }
        Control::Palette(kind) => {
            state.brush = Placement::Primitive(kind);
            state.tool = Tool::Build;
            state.status = "Click to place, or drag this object from the palette.";
        }
        Control::Run => {
            state.catalog.close();
            match RunSession::new(&state.document) {
                Ok(run) => {
                    state.run = Some(run);
                    state.mode = Mode::Preview;
                    state.status =
                        "Live rigid-body mechanics. Stop returns to the untouched authoring scene.";
                }
                Err(error) => state.notice = Some(format!("Cannot start physics: {error}")),
            }
        }
        Control::Back => {
            state.mode = Mode::Editor;
            state.run = None;
            state.status = "Run discarded. Back in the unchanged authoring scene.";
        }
        Control::Environment => {
            state.catalog.close();
            state.environment_open = true;
            if let Some(run) = &mut state.run {
                run.discard_elapsed();
            }
        }
        Control::CloseEnvironment => {
            state.environment_open = false;
            if let Some(run) = &mut state.run {
                run.discard_elapsed();
            }
        }
        Control::Pause => {
            if let Some(run) = &mut state.run {
                run.pause(!run.paused);
            }
        }
        Control::Slow | Control::Normal | Control::Fast => {
            if let Some(run) = &mut state.run {
                run.set_speed(match control {
                    Control::Slow => PlaybackSpeed::Quarter,
                    Control::Fast => PlaybackSpeed::Fast,
                    _ => PlaybackSpeed::Normal,
                });
            }
        }
        Control::Pendulum | Control::SpringPair => {
            let origin = Point::new(state.camera.center.x, state.camera.center.y + 1.5);
            let result = if control == Control::Pendulum {
                state.document.add_pendulum(origin)
            } else {
                state.document.add_spring_pair(origin)
            };
            let result = result.map(|id| {
                state.select_one(Some(id));
                state.tool = Tool::Select;
            });
            report(state, result);
        }
        control if control.is_environment() => {
            let mut environment = state.environment();
            match control {
                Control::GravityLeft => environment.gravity_m_s2.x -= 1.0,
                Control::GravityRight => environment.gravity_m_s2.x += 1.0,
                Control::GravityDown => environment.gravity_m_s2.y -= 1.0,
                Control::GravityUp => environment.gravity_m_s2.y += 1.0,
                Control::EarthGravity => {
                    environment.gravity_m_s2 = PhysicsEnvironment::default().gravity_m_s2
                }
                Control::ZeroGravity => environment.gravity_m_s2 = Point::default(),
                Control::DragDown => {
                    environment.linear_drag_per_s = (environment.linear_drag_per_s - 0.1).max(0.0)
                }
                Control::DragUp => environment.linear_drag_per_s += 0.1,
                _ => return,
            }
            if state.mode == Mode::Preview {
                if let Some(run) = &mut state.run {
                    match run.set_environment(environment) {
                        Ok(()) => {
                            state.status = "Run environment changed at a step boundary; authoring settings unchanged."
                        }
                        Err(error) => state.notice = Some(format!("Environment rejected: {error}")),
                    }
                }
            } else {
                let result = state.document.set_environment(environment);
                report(state, result);
            }
        }
        Control::Home => {
            state.camera = Camera::default();
            state.status = "Camera reset. Scene objects were not moved.";
        }
        Control::Snap => state.snap = !state.snap,
        Control::Undo => {
            state.document.undo();
            state.select_one(None);
            state.status = "Undo. Camera and view mode are not part of document history.";
        }
        Control::Redo => {
            state.document.redo();
            state.select_one(None);
            state.status = "Redo.";
        }
        _ => {
            if control == Control::Delete && state.selection.len() > 1 {
                let ids: Vec<_> = state.selection.iter().copied().collect();
                let result = state.document.remove_many(&ids);
                if result.is_ok() {
                    state.select_one(None);
                }
                report(state, result);
                return;
            }
            let Some(object) = state.selected_object().cloned() else {
                return;
            };
            let result = match control {
                Control::Delete => state.document.remove(object.id),
                Control::Duplicate => state.document.duplicate(object.id).map(|id| {
                    state.select_one(Some(id));
                }),
                Control::MassDown => state.document.set_mass(object.id, object.mass_kg / 2.0),
                Control::MassUp => state.document.set_mass(object.id, object.mass_kg * 2.0),
                Control::SizeDown => state.document.set_size(object.id, object.size_m * 0.5),
                Control::SizeUp => state.document.set_size(object.id, object.size_m * 2.0),
                Control::HeightDown => state.document.set_height(object.id, object.height_m * 0.5),
                Control::HeightUp => state.document.set_height(object.id, object.height_m * 2.0),
                Control::Fixed => state.document.set_fixed(object.id, !object.fixed),
                Control::RestitutionDown => state
                    .document
                    .set_restitution(object.id, (object.restitution - 0.1).max(0.0)),
                Control::RestitutionUp => state
                    .document
                    .set_restitution(object.id, (object.restitution + 0.1).min(1.0)),
                Control::RotateLeft => state.document.rotate(object.id, -15.0),
                Control::RotateRight => state.document.rotate(object.id, 15.0),
                _ => return,
            };
            report(state, result);
        }
    }
}
