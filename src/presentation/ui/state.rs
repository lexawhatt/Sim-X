use winit::keyboard::KeyCode;

use super::{
    catalog::{MechanicsTool, PhysSubdomain, Screen, TopDomain},
    geometry::Point,
    layout::{InteractionTarget, UiLayout},
};

const TIME_EASTER_EGG_CODE: [u8; 7] = [1, 0, 4, 8, 5, 9, 6];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::presentation) enum Notice {
    DomainUnavailable(TopDomain),
    SubdomainUnavailable(PhysSubdomain),
    ToolAwaitingDomain(MechanicsTool),
}

#[derive(Debug)]
pub(in crate::presentation) struct AnimationState {
    pub(in crate::presentation) domain_hover: [f32; 4],
    pub(in crate::presentation) phys_hover: [f32; 7],
    pub(in crate::presentation) tool_hover: [f32; 4],
    pub(in crate::presentation) time_code_hover: f32,
    pub(in crate::presentation) back_hover: f32,
    pub(in crate::presentation) reset_hover: f32,
}

impl Default for AnimationState {
    fn default() -> Self {
        Self {
            domain_hover: [0.0; 4],
            phys_hover: [0.0; 7],
            tool_hover: [0.0; 4],
            time_code_hover: 0.0,
            back_hover: 0.0,
            reset_hover: 0.0,
        }
    }
}

impl AnimationState {
    fn update(&mut self, screen: Screen, hovered: Option<InteractionTarget>, elapsed: f32) {
        for domain in TopDomain::ALL {
            let target =
                matches!(hovered, Some(InteractionTarget::Domain(value)) if value == domain);
            smooth_towards(
                &mut self.domain_hover[domain.index()],
                if target { 1.0 } else { 0.0 },
                elapsed,
            );
        }
        for subdomain in PhysSubdomain::ALL {
            let target = matches!(hovered, Some(InteractionTarget::PhysSubdomain(value)) if value == subdomain);
            smooth_towards(
                &mut self.phys_hover[subdomain.index()],
                if target { 1.0 } else { 0.0 },
                elapsed,
            );
        }
        for tool in MechanicsTool::ALL {
            let target =
                matches!(hovered, Some(InteractionTarget::MechanicsTool(value)) if value == tool);
            smooth_towards(
                &mut self.tool_hover[tool.index()],
                if target { 1.0 } else { 0.0 },
                elapsed,
            );
        }
        smooth_towards(
            &mut self.time_code_hover,
            if hovered == Some(InteractionTarget::TimeCodeHotspot) {
                1.0
            } else {
                0.0
            },
            elapsed,
        );
        let back_target = match screen {
            Screen::Domains => false,
            Screen::PhysSubdomains => hovered == Some(InteractionTarget::BackToDomains),
            Screen::MechanicsEditor => hovered == Some(InteractionTarget::BackToPhys),
            Screen::TimeEasterEgg => hovered == Some(InteractionTarget::BackToDomains),
        };
        smooth_towards(
            &mut self.back_hover,
            if back_target { 1.0 } else { 0.0 },
            elapsed,
        );
        smooth_towards(
            &mut self.reset_hover,
            if hovered == Some(InteractionTarget::EditorReset) {
                1.0
            } else {
                0.0
            },
            elapsed,
        );
    }
}

#[derive(Debug)]
pub(in crate::presentation) struct UiState {
    pub(in crate::presentation) screen: Screen,
    pub(in crate::presentation) notice: Option<Notice>,
    pub(in crate::presentation) selected_tool: MechanicsTool,
    pub(in crate::presentation) pendulum_anchor: Point,
    pub(in crate::presentation) animations: AnimationState,
    pub(in crate::presentation) time_code_progress: usize,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            screen: Screen::Domains,
            notice: None,
            selected_tool: MechanicsTool::Pendulum,
            pendulum_anchor: Point::new(0.5, 0.24),
            animations: AnimationState::default(),
            time_code_progress: 0,
        }
    }
}

impl UiState {
    pub(in crate::presentation) fn update_animations(
        &mut self,
        hovered: Option<InteractionTarget>,
        elapsed: f32,
    ) {
        self.animations.update(self.screen, hovered, elapsed);
    }

    pub(in crate::presentation) fn activate(
        &mut self,
        target: InteractionTarget,
        pointer: Point,
        layout: UiLayout,
    ) {
        match target {
            InteractionTarget::Domain(TopDomain::Phys) => {
                self.screen = Screen::PhysSubdomains;
                self.notice = None;
            }
            InteractionTarget::Domain(domain) => {
                self.notice = Some(Notice::DomainUnavailable(domain));
            }
            InteractionTarget::TimeCodeHotspot => {}
            InteractionTarget::PhysSubdomain(PhysSubdomain::Mechanics) => {
                self.screen = Screen::MechanicsEditor;
                self.notice = None;
            }
            InteractionTarget::PhysSubdomain(subdomain) => {
                self.notice = Some(Notice::SubdomainUnavailable(subdomain));
            }
            InteractionTarget::BackToDomains => {
                self.screen = Screen::Domains;
                self.notice = None;
            }
            InteractionTarget::BackToPhys => {
                self.screen = Screen::PhysSubdomains;
                self.notice = None;
            }
            InteractionTarget::MechanicsTool(tool) => {
                self.selected_tool = tool;
                self.notice =
                    (tool != MechanicsTool::Pendulum).then_some(Notice::ToolAwaitingDomain(tool));
            }
            InteractionTarget::EditorCanvas => {
                if self.selected_tool == MechanicsTool::Pendulum {
                    self.pendulum_anchor = layout.editor_canvas().normalized_point(pointer);
                    self.notice = None;
                }
            }
            InteractionTarget::EditorReset => {
                self.selected_tool = MechanicsTool::Pendulum;
                self.pendulum_anchor = Point::new(0.5, 0.24);
                self.notice = None;
            }
        }
    }

    pub(in crate::presentation) fn handle_key(
        &mut self,
        key_code: KeyCode,
        layout: UiLayout,
        hovered: Option<InteractionTarget>,
    ) {
        if self.advance_time_code(key_code, hovered) {
            return;
        }

        let target = match (self.screen, key_code) {
            (Screen::Domains, KeyCode::Digit1) => Some(InteractionTarget::Domain(TopDomain::Math)),
            (Screen::Domains, KeyCode::Digit2 | KeyCode::Enter) => {
                Some(InteractionTarget::Domain(TopDomain::Phys))
            }
            (Screen::Domains, KeyCode::Digit3) => Some(InteractionTarget::Domain(TopDomain::Chem)),
            (Screen::Domains, KeyCode::Digit4) => Some(InteractionTarget::Domain(TopDomain::Biol)),
            (Screen::PhysSubdomains, KeyCode::Digit1 | KeyCode::Enter) => {
                Some(InteractionTarget::PhysSubdomain(PhysSubdomain::Mechanics))
            }
            (Screen::PhysSubdomains, KeyCode::Digit2) => Some(InteractionTarget::PhysSubdomain(
                PhysSubdomain::Thermodynamics,
            )),
            (Screen::PhysSubdomains, KeyCode::Digit3) => Some(InteractionTarget::PhysSubdomain(
                PhysSubdomain::WavesAndOptics,
            )),
            (Screen::PhysSubdomains, KeyCode::Digit4) => Some(InteractionTarget::PhysSubdomain(
                PhysSubdomain::Electromagnetism,
            )),
            (Screen::PhysSubdomains, KeyCode::Digit5) => {
                Some(InteractionTarget::PhysSubdomain(PhysSubdomain::Relativity))
            }
            (Screen::PhysSubdomains, KeyCode::Digit6) => Some(InteractionTarget::PhysSubdomain(
                PhysSubdomain::FluidDynamics,
            )),
            (Screen::PhysSubdomains, KeyCode::Digit7) => {
                Some(InteractionTarget::PhysSubdomain(PhysSubdomain::Sandbox))
            }
            (Screen::MechanicsEditor, KeyCode::KeyR) => Some(InteractionTarget::EditorReset),
            (_, KeyCode::Escape) => {
                self.go_back();
                None
            }
            _ => None,
        };

        if let Some(target) = target {
            self.activate(target, Point::default(), layout);
        }
    }

    fn advance_time_code(&mut self, key_code: KeyCode, hovered: Option<InteractionTarget>) -> bool {
        if self.screen != Screen::Domains || hovered != Some(InteractionTarget::TimeCodeHotspot) {
            self.time_code_progress = 0;
            return false;
        }

        let Some(digit) = key_code_digit(key_code) else {
            self.time_code_progress = 0;
            return false;
        };
        if digit == TIME_EASTER_EGG_CODE[self.time_code_progress] {
            self.time_code_progress += 1;
        } else {
            self.time_code_progress = usize::from(digit == TIME_EASTER_EGG_CODE[0]);
        }

        if self.time_code_progress == TIME_EASTER_EGG_CODE.len() {
            self.time_code_progress = 0;
            self.screen = Screen::TimeEasterEgg;
            self.notice = None;
            return true;
        }
        true
    }

    fn go_back(&mut self) {
        match self.screen {
            Screen::Domains => {}
            Screen::PhysSubdomains => self.screen = Screen::Domains,
            Screen::MechanicsEditor => self.screen = Screen::PhysSubdomains,
            Screen::TimeEasterEgg => self.screen = Screen::Domains,
        }
        self.notice = None;
    }
}

fn key_code_digit(key_code: KeyCode) -> Option<u8> {
    match key_code {
        KeyCode::Digit0 | KeyCode::Numpad0 => Some(0),
        KeyCode::Digit1 | KeyCode::Numpad1 => Some(1),
        KeyCode::Digit2 | KeyCode::Numpad2 => Some(2),
        KeyCode::Digit3 | KeyCode::Numpad3 => Some(3),
        KeyCode::Digit4 | KeyCode::Numpad4 => Some(4),
        KeyCode::Digit5 | KeyCode::Numpad5 => Some(5),
        KeyCode::Digit6 | KeyCode::Numpad6 => Some(6),
        KeyCode::Digit7 | KeyCode::Numpad7 => Some(7),
        KeyCode::Digit8 | KeyCode::Numpad8 => Some(8),
        KeyCode::Digit9 | KeyCode::Numpad9 => Some(9),
        _ => None,
    }
}

fn smooth_towards(current: &mut f32, target: f32, elapsed: f32) {
    let response = 1.0 - (-elapsed.clamp(0.0, 0.05) * 18.0).exp();
    *current += (target - *current) * response;
    if (*current - target).abs() < 0.001 {
        *current = target;
    }
}

#[cfg(test)]
mod tests {
    use super::{Notice, UiState};
    use crate::presentation::ui::{
        catalog::{MechanicsTool, PhysSubdomain, Screen, TopDomain},
        geometry::Point,
        layout::{InteractionTarget, UiLayout},
    };
    use winit::keyboard::KeyCode;

    #[test]
    fn active_navigation_reaches_the_mechanics_editor() {
        let layout = UiLayout::new(1920.0, 1080.0);
        let mut state = UiState::default();

        state.activate(
            InteractionTarget::Domain(TopDomain::Phys),
            Point::default(),
            layout,
        );
        assert_eq!(state.screen, Screen::PhysSubdomains);

        state.activate(
            InteractionTarget::PhysSubdomain(PhysSubdomain::Mechanics),
            Point::default(),
            layout,
        );
        assert_eq!(state.screen, Screen::MechanicsEditor);
    }

    #[test]
    fn inactive_subdomain_stays_on_catalog_and_reports_notice() {
        let layout = UiLayout::new(1920.0, 1080.0);
        let mut state = UiState {
            screen: Screen::PhysSubdomains,
            ..UiState::default()
        };

        state.activate(
            InteractionTarget::PhysSubdomain(PhysSubdomain::Thermodynamics),
            Point::default(),
            layout,
        );

        assert_eq!(state.screen, Screen::PhysSubdomains);
        assert_eq!(
            state.notice,
            Some(Notice::SubdomainUnavailable(PhysSubdomain::Thermodynamics))
        );
    }

    #[test]
    fn pendulum_placement_uses_canvas_normalized_coordinates() {
        let layout = UiLayout::new(1920.0, 1080.0);
        let canvas = layout.editor_canvas();
        let target = Point::new(
            canvas.min.x + canvas.width() * 0.25,
            canvas.min.y + canvas.height() * 0.75,
        );
        let mut state = UiState {
            screen: Screen::MechanicsEditor,
            selected_tool: MechanicsTool::Pendulum,
            ..UiState::default()
        };

        state.activate(InteractionTarget::EditorCanvas, target, layout);

        assert!((state.pendulum_anchor.x - 0.25).abs() < f32::EPSILON);
        assert!((state.pendulum_anchor.y - 0.75).abs() < f32::EPSILON);
    }

    #[test]
    fn time_code_is_consumed_only_while_the_hotspot_is_hovered() {
        let layout = UiLayout::new(1920.0, 1080.0);
        let mut state = UiState::default();

        state.handle_key(KeyCode::Digit1, layout, None);
        assert_eq!(
            state.notice,
            Some(Notice::DomainUnavailable(TopDomain::Math))
        );

        state.notice = None;
        for key in [
            KeyCode::Digit1,
            KeyCode::Digit0,
            KeyCode::Digit4,
            KeyCode::Digit8,
            KeyCode::Digit5,
            KeyCode::Digit9,
            KeyCode::Digit6,
        ] {
            state.handle_key(key, layout, Some(InteractionTarget::TimeCodeHotspot));
        }

        assert_eq!(state.screen, Screen::TimeEasterEgg);
        assert_eq!(state.notice, None);
    }

    #[test]
    fn leaving_the_hotspot_resets_an_incomplete_time_code() {
        let layout = UiLayout::new(1920.0, 1080.0);
        let mut state = UiState::default();
        let hotspot = Some(InteractionTarget::TimeCodeHotspot);

        state.handle_key(KeyCode::Digit1, layout, hotspot);
        state.handle_key(KeyCode::Digit0, layout, hotspot);
        state.handle_key(KeyCode::Digit8, layout, None);

        for key in [
            KeyCode::Digit4,
            KeyCode::Digit8,
            KeyCode::Digit5,
            KeyCode::Digit9,
            KeyCode::Digit6,
        ] {
            state.handle_key(key, layout, hotspot);
        }

        assert_eq!(state.screen, Screen::Domains);
    }
}
