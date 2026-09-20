//! One-window handoff from project creation to a fresh authoring World.

use sim_logic::prelude::*;

use crate::{menu::state::MenuState, phys_editor::state::EditorState};

#[cfg(test)]
mod tests;

pub(crate) struct Navigation {
    pub(crate) editor: WorldFactoryId,
    pub(crate) pending: Option<ProjectEntry>,
}

pub(crate) struct ProjectEntry {
    name: String,
    fullscreen: bool,
}

pub(crate) fn open_project(
    menu: Option<ResMut<MenuState>>,
    mut navigation: AppResMut<Navigation>,
    mut commands: Commands,
) -> LogicResult {
    let Some(mut menu) = menu else {
        return Ok(());
    };
    let Some(name) = menu.pending_project.take() else {
        return Ok(());
    };
    let intent = commands.new_transition_intent()?;
    commands.replace_world(intent, navigation.editor)?;
    navigation.pending = Some(ProjectEntry {
        name,
        fullscreen: menu.fullscreen_requested,
    });
    Ok(())
}

/// Startup is deliberately pure in Logic; application payloads enter only
/// after the candidate World has committed, before its first input/view update.
pub(crate) fn initialize_project(
    editor: Option<ResMut<EditorState>>,
    mut navigation: AppResMut<Navigation>,
) {
    let Some(mut editor) = editor else {
        return;
    };
    if let Some(entry) = navigation.pending.take() {
        editor.project_name = entry.name;
        editor.fullscreen_requested = entry.fullscreen;
    }
}
