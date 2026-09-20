use sim_logic::prelude::*;

/// Shared physical input vocabulary; each active screen owns its interpretation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AppAction {
    /// Selects, stamps or confirms a left-button gesture.
    Point,
    /// Pans the viewport with the middle mouse button.
    Pan,
    /// Tracks physical Ctrl keys for document shortcuts.
    Control,
    /// Tracks physical Shift keys for redo.
    Shift,
    /// Select tool.
    Select,
    /// Build tool.
    Build,
    /// Erase tool.
    Erase,
    /// Undo when Ctrl is held; Ctrl+Shift+Z redoes.
    Undo,
    /// Redo when Ctrl is held.
    Redo,
    /// Deletes the selected object in Editor only.
    Delete,
    /// Duplicates the selection when Ctrl is held.
    Duplicate,
    /// Toggles grid snapping in Editor.
    Snap,
    /// Restores the initial camera framing.
    Home,
    /// Cancels a gesture, clears selection, or returns from Preview.
    Escape,
    /// Starts an independent fixed-step Physics run in View.
    Run,
    /// Requests fullscreen/windowed mode.
    Fullscreen,
    /// Creates a rigid relationship by selecting two existing bodies.
    Rod,
    /// Creates a Hooke spring between two existing bodies.
    Spring,
    /// Opens the scene/run environment panel.
    Environment,
    /// Toggles playback in View only.
    Pause,
    /// Searches the current object catalog with Ctrl+F.
    CatalogSearch,
    /// Physical ASCII/navigation input owned exclusively by the open catalog.
    CatalogInput,
}

pub(crate) fn bind(app: &mut Application<AppAction>) -> LogicResult {
    app.bind_mouse_button(MouseButton::Left, AppAction::Point)?;
    app.bind_mouse_button(MouseButton::Middle, AppAction::Pan)?;
    for (key, action) in [
        (PhysicalKeyCode::ControlLeft, AppAction::Control),
        (PhysicalKeyCode::ControlRight, AppAction::Control),
        (PhysicalKeyCode::ShiftLeft, AppAction::Shift),
        (PhysicalKeyCode::ShiftRight, AppAction::Shift),
        (PhysicalKeyCode::Digit1, AppAction::Select),
        (PhysicalKeyCode::Digit2, AppAction::Build),
        (PhysicalKeyCode::Digit3, AppAction::Erase),
        (PhysicalKeyCode::KeyZ, AppAction::Undo),
        (PhysicalKeyCode::KeyY, AppAction::Redo),
        (PhysicalKeyCode::Delete, AppAction::Delete),
        (PhysicalKeyCode::KeyD, AppAction::Duplicate),
        (PhysicalKeyCode::KeyG, AppAction::Snap),
        (PhysicalKeyCode::Home, AppAction::Home),
        (PhysicalKeyCode::Escape, AppAction::Escape),
        (PhysicalKeyCode::F5, AppAction::Run),
        (PhysicalKeyCode::F11, AppAction::Fullscreen),
        (PhysicalKeyCode::Digit4, AppAction::Rod),
        (PhysicalKeyCode::Digit5, AppAction::Spring),
        (PhysicalKeyCode::KeyE, AppAction::Environment),
        (PhysicalKeyCode::Space, AppAction::Pause),
        (PhysicalKeyCode::KeyF, AppAction::CatalogSearch),
    ] {
        app.bind_key(key, action)?;
    }
    use PhysicalKeyCode::*;
    for key in [
        KeyA, KeyB, KeyC, KeyH, KeyI, KeyJ, KeyK, KeyL, KeyM, KeyN, KeyO, KeyP, KeyQ, KeyR, KeyS,
        KeyT, KeyU, KeyV, KeyW, KeyX, Digit0, Digit6, Digit7, Digit8, Digit9, Minus, Slash,
        Backspace, Enter, Tab, ArrowLeft, ArrowRight, ArrowUp, ArrowDown,
    ] {
        app.bind_key(key, AppAction::CatalogInput)?;
    }
    Ok(())
}
