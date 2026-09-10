use winit::keyboard::KeyCode;

use crate::foundation::EntityId;

use super::{
    PhysicsEditorCommand, PhysicsEditorOutcome,
    catalog::{
        MainMenuItem, MechanicsTool, PhysSubdomain, PlaybackRate, ProjectTemplate, Screen,
        SocialLink, TopDomain, ViewExitChoice,
    },
    geometry::Point,
    layout::{InteractionTarget, UiLayout},
};

const TIME_EASTER_EGG_CODE: [u8; 7] = [1, 0, 4, 8, 5, 9, 6];

#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::presentation) enum Notice {
    DomainUnavailable(TopDomain),
    SubdomainUnavailable(PhysSubdomain),
    ProjectUnavailable(ProjectTemplate),
    ToolAwaitingDomain(MechanicsTool),
    ExternalLinkFailed(SocialLink),
    ViewStateApplied,
    ViewStateDiscarded,
    EditorActionCommitted,
    EditorActionFailed,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::presentation) enum UiAction {
    OpenExternalLink(SocialLink),
    ExitRequested,
    OpenPhysicsProject {
        subdomain: PhysSubdomain,
        template: ProjectTemplate,
    },
    ResetPhysicsProject,
    EnterPhysicsView,
    ApplyPhysicsView,
    DiscardPhysicsView,
    ClosePhysicsProject,
    SetPlaybackRate(PlaybackRate),
    EditPhysics(PhysicsEditorCommand),
}

#[derive(Debug)]
pub(in crate::presentation) struct AnimationState {
    pub(in crate::presentation) main_hover: [f32; 3],
    pub(in crate::presentation) social_hover: [f32; 3],
    pub(in crate::presentation) domain_hover: [f32; 4],
    pub(in crate::presentation) phys_hover: [f32; 7],
    pub(in crate::presentation) project_hover: [f32; 3],
    pub(in crate::presentation) tool_hover: [f32; 4],
    pub(in crate::presentation) playback_hover: [f32; 4],
    pub(in crate::presentation) time_code_hover: f32,
    pub(in crate::presentation) back_hover: f32,
    pub(in crate::presentation) reset_hover: f32,
    pub(in crate::presentation) view_simulation_hover: f32,
    pub(in crate::presentation) view_exit_hover: [f32; 3],
}

impl Default for AnimationState {
    fn default() -> Self {
        Self {
            main_hover: [0.0; 3],
            social_hover: [0.0; 3],
            domain_hover: [0.0; 4],
            phys_hover: [0.0; 7],
            project_hover: [0.0; 3],
            tool_hover: [0.0; 4],
            playback_hover: [0.0; 4],
            time_code_hover: 0.0,
            back_hover: 0.0,
            reset_hover: 0.0,
            view_simulation_hover: 0.0,
            view_exit_hover: [0.0; 3],
        }
    }
}

impl AnimationState {
    fn update(&mut self, hovered: Option<InteractionTarget>, elapsed: f32) {
        for item in MainMenuItem::ALL {
            update_hover(
                &mut self.main_hover[item.index()],
                matches!(hovered, Some(InteractionTarget::MainMenu(value)) if value == item),
                elapsed,
            );
        }
        for link in SocialLink::ALL {
            update_hover(
                &mut self.social_hover[link.index()],
                matches!(hovered, Some(InteractionTarget::SocialLink(value)) if value == link),
                elapsed,
            );
        }
        for domain in TopDomain::ALL {
            update_hover(
                &mut self.domain_hover[domain.index()],
                matches!(hovered, Some(InteractionTarget::Domain(value)) if value == domain),
                elapsed,
            );
        }
        for subdomain in PhysSubdomain::ALL {
            update_hover(
                &mut self.phys_hover[subdomain.index()],
                matches!(hovered, Some(InteractionTarget::PhysSubdomain(value)) if value == subdomain),
                elapsed,
            );
        }
        for project in ProjectTemplate::ALL {
            update_hover(
                &mut self.project_hover[project.index()],
                matches!(hovered, Some(InteractionTarget::Project(value)) if value == project),
                elapsed,
            );
        }
        for tool in MechanicsTool::ALL {
            update_hover(
                &mut self.tool_hover[tool.index()],
                matches!(hovered, Some(InteractionTarget::MechanicsTool(value)) if value == tool),
                elapsed,
            );
        }
        for rate in PlaybackRate::ALL {
            update_hover(
                &mut self.playback_hover[rate.index()],
                matches!(hovered, Some(InteractionTarget::Playback(value)) if value == rate),
                elapsed,
            );
        }
        update_hover(
            &mut self.time_code_hover,
            hovered == Some(InteractionTarget::TimeCodeHotspot),
            elapsed,
        );
        update_hover(
            &mut self.back_hover,
            hovered == Some(InteractionTarget::Back),
            elapsed,
        );
        update_hover(
            &mut self.reset_hover,
            hovered == Some(InteractionTarget::LabReset),
            elapsed,
        );
        update_hover(
            &mut self.view_simulation_hover,
            hovered == Some(InteractionTarget::ViewSimulation),
            elapsed,
        );
        for choice in ViewExitChoice::ALL {
            update_hover(
                &mut self.view_exit_hover[choice.index()],
                matches!(hovered, Some(InteractionTarget::ViewExitChoice(value)) if value == choice),
                elapsed,
            );
        }
    }
}

#[derive(Debug)]
pub(in crate::presentation) struct UiState {
    pub(in crate::presentation) screen: Screen,
    pub(in crate::presentation) notice: Option<Notice>,
    pub(in crate::presentation) selected_subdomain: PhysSubdomain,
    pub(in crate::presentation) selected_project: ProjectTemplate,
    pub(in crate::presentation) selected_tool: MechanicsTool,
    pub(in crate::presentation) selected_body: Option<EntityId>,
    pub(in crate::presentation) playback_rate: PlaybackRate,
    pub(in crate::presentation) view_exit_resume_rate: PlaybackRate,
    pub(in crate::presentation) force_drag_preview: Option<(Point, Point)>,
    pub(in crate::presentation) animations: AnimationState,
    pub(in crate::presentation) time_code_progress: usize,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            screen: Screen::MainMenu,
            notice: None,
            selected_subdomain: PhysSubdomain::Mechanics,
            selected_project: ProjectTemplate::PrimaryLab,
            selected_tool: MechanicsTool::Body,
            selected_body: None,
            playback_rate: PlaybackRate::Paused,
            view_exit_resume_rate: PlaybackRate::Paused,
            force_drag_preview: None,
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
        self.animations.update(hovered, elapsed);
    }

    pub(in crate::presentation) fn activate(
        &mut self,
        target: InteractionTarget,
        _pointer: Point,
        _layout: UiLayout,
    ) -> Option<UiAction> {
        match target {
            InteractionTarget::MainMenu(MainMenuItem::Domains) => {
                self.screen = Screen::Domains;
                self.notice = None;
            }
            InteractionTarget::MainMenu(MainMenuItem::Settings) => {
                self.screen = Screen::Settings;
                self.notice = None;
            }
            InteractionTarget::MainMenu(MainMenuItem::Quit) => {
                return Some(UiAction::ExitRequested);
            }
            InteractionTarget::SocialLink(link) => {
                return Some(UiAction::OpenExternalLink(link));
            }
            InteractionTarget::Domain(TopDomain::Phys) => {
                self.screen = Screen::PhysSubdomains;
                self.notice = None;
            }
            InteractionTarget::Domain(domain) => {
                self.notice = Some(Notice::DomainUnavailable(domain));
            }
            InteractionTarget::TimeCodeHotspot => {}
            InteractionTarget::PhysSubdomain(subdomain) => {
                self.selected_subdomain = subdomain;
                self.screen = Screen::Projects;
                self.notice =
                    (!subdomain.has_runtime()).then_some(Notice::SubdomainUnavailable(subdomain));
            }
            InteractionTarget::Project(project) => {
                if !self.selected_subdomain.has_runtime() {
                    self.notice = Some(Notice::SubdomainUnavailable(self.selected_subdomain));
                } else if !project.is_available(self.selected_subdomain) {
                    self.notice = Some(Notice::ProjectUnavailable(project));
                } else {
                    self.selected_project = project;
                    self.selected_tool = MechanicsTool::Body;
                    self.selected_body = None;
                    self.screen = Screen::PhysicsEditor;
                    self.notice = None;
                    return Some(UiAction::OpenPhysicsProject {
                        subdomain: self.selected_subdomain,
                        template: project,
                    });
                }
            }
            InteractionTarget::Back => {
                if self.screen == Screen::PhysicsView {
                    self.view_exit_resume_rate = self.playback_rate;
                    self.screen = Screen::PhysicsViewExit;
                    self.notice = None;
                    return Some(UiAction::SetPlaybackRate(PlaybackRate::Paused));
                }
                let closing_project = self.screen == Screen::PhysicsEditor;
                self.go_back();
                if closing_project {
                    return Some(UiAction::ClosePhysicsProject);
                }
            }
            InteractionTarget::ViewSimulation => {
                if self.screen == Screen::PhysicsEditor {
                    self.screen = Screen::PhysicsView;
                    self.notice = None;
                    return Some(UiAction::EnterPhysicsView);
                }
            }
            InteractionTarget::MechanicsTool(tool) => {
                if self.screen == Screen::PhysicsEditor {
                    if matches!(tool, MechanicsTool::Body | MechanicsTool::Force) {
                        self.selected_tool = tool;
                        self.notice = None;
                    } else {
                        self.notice = Some(Notice::ToolAwaitingDomain(tool));
                    }
                }
            }
            InteractionTarget::InspectorMassDecrease => {
                if self.screen == Screen::PhysicsEditor
                    && let Some(entity) = self.selected_body
                {
                    return Some(UiAction::EditPhysics(
                        PhysicsEditorCommand::ScaleMechanicsBodyMass { entity, scale: 0.5 },
                    ));
                }
            }
            InteractionTarget::InspectorMassIncrease => {
                if self.screen == Screen::PhysicsEditor
                    && let Some(entity) = self.selected_body
                {
                    return Some(UiAction::EditPhysics(
                        PhysicsEditorCommand::ScaleMechanicsBodyMass { entity, scale: 2.0 },
                    ));
                }
            }
            InteractionTarget::EditorCanvas => {}
            InteractionTarget::LabReset => {
                self.selected_tool = MechanicsTool::Body;
                self.selected_body = None;
                self.force_drag_preview = None;
                self.notice = None;
                return Some(UiAction::ResetPhysicsProject);
            }
            InteractionTarget::Playback(rate) => {
                if self.screen == Screen::PhysicsView && self.selected_subdomain.supports_playback()
                {
                    self.notice = None;
                    return Some(UiAction::SetPlaybackRate(rate));
                }
            }
            InteractionTarget::ViewExitChoice(choice) => {
                if self.screen == Screen::PhysicsViewExit {
                    match choice {
                        ViewExitChoice::Apply => {
                            self.screen = Screen::PhysicsEditor;
                            self.notice = Some(Notice::ViewStateApplied);
                            return Some(UiAction::ApplyPhysicsView);
                        }
                        ViewExitChoice::Discard => {
                            self.screen = Screen::PhysicsEditor;
                            self.notice = Some(Notice::ViewStateDiscarded);
                            return Some(UiAction::DiscardPhysicsView);
                        }
                        ViewExitChoice::Cancel => {
                            self.screen = Screen::PhysicsView;
                            self.notice = None;
                            return Some(UiAction::SetPlaybackRate(self.view_exit_resume_rate));
                        }
                    }
                }
            }
        }
        None
    }

    pub(in crate::presentation) fn handle_key(
        &mut self,
        key_code: KeyCode,
        layout: UiLayout,
        hovered: Option<InteractionTarget>,
    ) -> Option<UiAction> {
        if self.advance_time_code(key_code, hovered) {
            return None;
        }

        let target = match (self.screen, key_code) {
            (Screen::MainMenu, KeyCode::Digit1 | KeyCode::Enter) => {
                Some(InteractionTarget::MainMenu(MainMenuItem::Domains))
            }
            (Screen::MainMenu, KeyCode::Digit2 | KeyCode::KeyS) => {
                Some(InteractionTarget::MainMenu(MainMenuItem::Settings))
            }
            (Screen::MainMenu, KeyCode::KeyQ) => {
                Some(InteractionTarget::MainMenu(MainMenuItem::Quit))
            }
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
            (Screen::Projects, KeyCode::Digit1) => {
                Some(InteractionTarget::Project(ProjectTemplate::Blank))
            }
            (Screen::Projects, KeyCode::Digit2 | KeyCode::Enter) => {
                Some(InteractionTarget::Project(ProjectTemplate::PrimaryLab))
            }
            (Screen::Projects, KeyCode::Digit3) => {
                Some(InteractionTarget::Project(ProjectTemplate::SecondaryLab))
            }
            (Screen::PhysicsEditor | Screen::PhysicsView, KeyCode::KeyR) => {
                Some(InteractionTarget::LabReset)
            }
            (Screen::PhysicsEditor, KeyCode::KeyV | KeyCode::Enter) => {
                Some(InteractionTarget::ViewSimulation)
            }
            (Screen::PhysicsView, KeyCode::Space)
                if self.selected_subdomain.supports_playback() =>
            {
                let rate = if self.playback_rate == PlaybackRate::Paused {
                    PlaybackRate::Normal
                } else {
                    PlaybackRate::Paused
                };
                Some(InteractionTarget::Playback(rate))
            }
            (Screen::PhysicsViewExit, KeyCode::KeyY | KeyCode::Enter) => {
                Some(InteractionTarget::ViewExitChoice(ViewExitChoice::Apply))
            }
            (Screen::PhysicsViewExit, KeyCode::KeyN | KeyCode::KeyD) => {
                Some(InteractionTarget::ViewExitChoice(ViewExitChoice::Discard))
            }
            (Screen::PhysicsViewExit, KeyCode::Escape) => {
                Some(InteractionTarget::ViewExitChoice(ViewExitChoice::Cancel))
            }
            (Screen::MainMenu, KeyCode::Escape) => None,
            (_, KeyCode::Escape) => Some(InteractionTarget::Back),
            _ => None,
        };

        target.and_then(|target| self.activate(target, Point::default(), layout))
    }

    pub(in crate::presentation) fn report_external_link_failure(&mut self, link: SocialLink) {
        self.notice = Some(Notice::ExternalLinkFailed(link));
    }

    pub(in crate::presentation) fn select_body(&mut self, entity: Option<EntityId>) {
        self.selected_body = entity;
        self.notice = None;
    }

    pub(in crate::presentation) fn set_force_drag_preview(
        &mut self,
        preview: Option<(Point, Point)>,
    ) {
        self.force_drag_preview = preview;
    }

    pub(in crate::presentation) fn report_editor_outcome(&mut self, outcome: PhysicsEditorOutcome) {
        if let PhysicsEditorOutcome::BodyPlaced(entity) = outcome {
            self.selected_body = Some(entity);
        }
        self.notice = Some(Notice::EditorActionCommitted);
    }

    pub(in crate::presentation) fn report_editor_failure(&mut self) {
        self.notice = Some(Notice::EditorActionFailed);
    }

    pub(in crate::presentation) fn reject_project_open(&mut self) {
        self.screen = Screen::Projects;
        self.notice = Some(Notice::ProjectUnavailable(self.selected_project));
        self.selected_body = None;
    }

    pub(in crate::presentation) fn reject_view_entry(&mut self) {
        self.screen = Screen::PhysicsEditor;
        self.notice = Some(Notice::EditorActionFailed);
    }

    /// Synchronizes the UI projection from the app-owned playback authority.
    pub(in crate::presentation) fn sync_playback_rate(&mut self, rate: PlaybackRate) {
        self.playback_rate = rate;
    }

    fn advance_time_code(&mut self, key_code: KeyCode, hovered: Option<InteractionTarget>) -> bool {
        if self.screen != Screen::MainMenu || hovered != Some(InteractionTarget::TimeCodeHotspot) {
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
        }
        true
    }

    fn go_back(&mut self) {
        self.screen = match self.screen {
            Screen::MainMenu => Screen::MainMenu,
            Screen::Settings | Screen::Domains | Screen::TimeEasterEgg => Screen::MainMenu,
            Screen::PhysSubdomains => Screen::Domains,
            Screen::Projects => Screen::PhysSubdomains,
            Screen::PhysicsEditor => Screen::Projects,
            Screen::PhysicsView => Screen::PhysicsEditor,
            Screen::PhysicsViewExit => Screen::PhysicsView,
        };
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

fn update_hover(current: &mut f32, target: bool, elapsed: f32) {
    let target = if target { 1.0 } else { 0.0 };
    let response = 1.0 - (-elapsed.clamp(0.0, 0.05) * 18.0).exp();
    *current += (target - *current) * response;
    if (*current - target).abs() < 0.001 {
        *current = target;
    }
}

#[cfg(test)]
mod tests {
    use super::{Notice, UiAction, UiState};
    use crate::presentation::ui::{
        PhysicsEditorCommand,
        catalog::{
            MainMenuItem, MechanicsTool, PhysSubdomain, ProjectTemplate, Screen, SocialLink,
            TopDomain, ViewExitChoice,
        },
        geometry::Point,
        layout::{InteractionTarget, UiLayout},
    };
    use winit::keyboard::KeyCode;

    #[test]
    fn navigation_reaches_projects_before_the_mechanics_editor() {
        let layout = UiLayout::new(1920.0, 1080.0);
        let mut state = UiState::default();

        state.activate(
            InteractionTarget::MainMenu(MainMenuItem::Domains),
            Point::default(),
            layout,
        );
        assert_eq!(state.screen, Screen::Domains);
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
        assert_eq!(state.screen, Screen::Projects);
        state.activate(
            InteractionTarget::Project(ProjectTemplate::PrimaryLab),
            Point::default(),
            layout,
        );
        assert_eq!(state.screen, Screen::PhysicsEditor);

        state.activate(InteractionTarget::Back, Point::default(), layout);
        assert_eq!(state.screen, Screen::Projects);
    }

    #[test]
    fn inactive_subdomain_opens_its_project_level_but_not_a_lab() {
        let layout = UiLayout::new(1920.0, 1080.0);
        let mut state = UiState {
            screen: Screen::PhysSubdomains,
            ..UiState::default()
        };

        state.activate(
            InteractionTarget::PhysSubdomain(PhysSubdomain::Relativity),
            Point::default(),
            layout,
        );
        assert_eq!(state.screen, Screen::Projects);
        assert_eq!(
            state.notice,
            Some(Notice::SubdomainUnavailable(PhysSubdomain::Relativity))
        );

        state.activate(
            InteractionTarget::Project(ProjectTemplate::Blank),
            Point::default(),
            layout,
        );
        assert_eq!(state.screen, Screen::Projects);
    }

    #[test]
    fn every_active_physics_subdomain_opens_its_primary_lab() {
        let layout = UiLayout::new(1920.0, 1080.0);
        for subdomain in [
            PhysSubdomain::Mechanics,
            PhysSubdomain::Thermodynamics,
            PhysSubdomain::WavesAndOptics,
            PhysSubdomain::Electromagnetism,
        ] {
            let mut state = UiState {
                screen: Screen::PhysSubdomains,
                ..UiState::default()
            };
            state.activate(
                InteractionTarget::PhysSubdomain(subdomain),
                Point::default(),
                layout,
            );
            let action = state.activate(
                InteractionTarget::Project(ProjectTemplate::PrimaryLab),
                Point::default(),
                layout,
            );
            assert_eq!(state.screen, Screen::PhysicsEditor);
            assert_eq!(
                action,
                Some(UiAction::OpenPhysicsProject {
                    subdomain,
                    template: ProjectTemplate::PrimaryLab,
                })
            );
        }
    }

    #[test]
    fn playback_controls_emit_typed_rate_intents() {
        let layout = UiLayout::new(1920.0, 1080.0);
        let mut state = UiState {
            screen: Screen::PhysicsView,
            ..UiState::default()
        };
        assert_eq!(
            state.activate(
                InteractionTarget::Playback(crate::presentation::ui::catalog::PlaybackRate::Slow),
                Point::default(),
                layout,
            ),
            Some(UiAction::SetPlaybackRate(
                crate::presentation::ui::catalog::PlaybackRate::Slow
            ))
        );
    }

    #[test]
    fn leaving_view_requires_an_explicit_apply_or_discard_choice() {
        let layout = UiLayout::new(1920.0, 1080.0);
        let mut state = UiState {
            screen: Screen::PhysicsEditor,
            ..UiState::default()
        };

        assert_eq!(
            state.activate(InteractionTarget::ViewSimulation, Point::default(), layout,),
            Some(UiAction::EnterPhysicsView)
        );
        assert_eq!(state.screen, Screen::PhysicsView);
        state.sync_playback_rate(crate::presentation::ui::PlaybackRate::Normal);

        assert_eq!(
            state.activate(InteractionTarget::Back, Point::default(), layout),
            Some(UiAction::SetPlaybackRate(
                crate::presentation::ui::PlaybackRate::Paused
            ))
        );
        assert_eq!(state.screen, Screen::PhysicsViewExit);
        assert_eq!(state.notice, None);

        assert_eq!(
            state.activate(
                InteractionTarget::ViewExitChoice(ViewExitChoice::Discard),
                Point::default(),
                layout,
            ),
            Some(UiAction::DiscardPhysicsView)
        );
        assert_eq!(state.screen, Screen::PhysicsEditor);
        assert_eq!(state.notice, Some(Notice::ViewStateDiscarded));
        state.sync_playback_rate(crate::presentation::ui::PlaybackRate::Paused);
        assert_eq!(
            state.playback_rate,
            crate::presentation::ui::PlaybackRate::Paused
        );
    }

    #[test]
    fn editor_rejects_playback_intents_even_if_called_directly() {
        let layout = UiLayout::new(1920.0, 1080.0);
        let mut state = UiState {
            screen: Screen::PhysicsEditor,
            ..UiState::default()
        };

        assert_eq!(
            state.activate(
                InteractionTarget::Playback(crate::presentation::ui::PlaybackRate::Fast),
                Point::default(),
                layout,
            ),
            None
        );
        assert_eq!(
            state.playback_rate,
            crate::presentation::ui::PlaybackRate::Paused
        );
    }

    #[test]
    fn locked_mechanics_tools_do_not_replace_the_active_editor_tool() {
        let layout = UiLayout::new(1920.0, 1080.0);
        let mut state = UiState {
            screen: Screen::PhysicsEditor,
            selected_tool: MechanicsTool::Force,
            ..UiState::default()
        };

        assert_eq!(
            state.activate(
                InteractionTarget::MechanicsTool(MechanicsTool::Pendulum),
                Point::default(),
                layout,
            ),
            None
        );
        assert_eq!(state.selected_tool, MechanicsTool::Force);
        assert_eq!(
            state.notice,
            Some(Notice::ToolAwaitingDomain(MechanicsTool::Pendulum))
        );
    }

    #[test]
    fn inspector_mass_controls_require_selection_and_emit_typed_scales() {
        let layout = UiLayout::new(1920.0, 1080.0);
        let mut state = UiState {
            screen: Screen::PhysicsEditor,
            ..UiState::default()
        };
        assert_eq!(
            state.activate(
                InteractionTarget::InspectorMassIncrease,
                Point::default(),
                layout,
            ),
            None
        );

        let entity = crate::foundation::EntityId::new(7).expect("entity");
        state.select_body(Some(entity));
        assert_eq!(
            state.activate(
                InteractionTarget::InspectorMassDecrease,
                Point::default(),
                layout,
            ),
            Some(UiAction::EditPhysics(
                PhysicsEditorCommand::ScaleMechanicsBodyMass { entity, scale: 0.5 }
            ))
        );
        assert_eq!(
            state.activate(
                InteractionTarget::InspectorMassIncrease,
                Point::default(),
                layout,
            ),
            Some(UiAction::EditPhysics(
                PhysicsEditorCommand::ScaleMechanicsBodyMass { entity, scale: 2.0 }
            ))
        );
    }

    #[test]
    fn canceling_view_exit_restores_the_previous_playback_intent() {
        let layout = UiLayout::new(1920.0, 1080.0);
        let mut state = UiState {
            screen: Screen::PhysicsView,
            ..UiState::default()
        };
        state.sync_playback_rate(crate::presentation::ui::PlaybackRate::Fast);
        assert_eq!(
            state.activate(InteractionTarget::Back, Point::default(), layout),
            Some(UiAction::SetPlaybackRate(
                crate::presentation::ui::PlaybackRate::Paused
            ))
        );

        assert_eq!(
            state.activate(
                InteractionTarget::ViewExitChoice(ViewExitChoice::Cancel),
                Point::default(),
                layout,
            ),
            Some(UiAction::SetPlaybackRate(
                crate::presentation::ui::PlaybackRate::Fast
            ))
        );
        assert_eq!(state.screen, Screen::PhysicsView);
    }

    #[test]
    fn static_electrostatics_rejects_pointer_and_keyboard_playback() {
        let layout = UiLayout::new(1920.0, 1080.0);
        let mut state = UiState {
            screen: Screen::PhysicsView,
            selected_subdomain: PhysSubdomain::Electromagnetism,
            ..UiState::default()
        };

        assert_eq!(
            state.activate(
                InteractionTarget::Playback(crate::presentation::ui::PlaybackRate::Fast),
                Point::default(),
                layout,
            ),
            None
        );
        assert_eq!(state.handle_key(KeyCode::Space, layout, None), None);
        assert_eq!(
            state.playback_rate,
            crate::presentation::ui::PlaybackRate::Paused
        );
    }

    #[test]
    fn social_link_activation_returns_an_external_action() {
        let layout = UiLayout::new(1920.0, 1080.0);
        let mut state = UiState::default();
        assert_eq!(
            state.activate(
                InteractionTarget::SocialLink(SocialLink::GitHub),
                Point::default(),
                layout,
            ),
            Some(UiAction::OpenExternalLink(SocialLink::GitHub))
        );
        assert_eq!(state.screen, Screen::MainMenu);
    }

    #[test]
    fn time_code_is_consumed_only_while_the_hotspot_is_hovered() {
        let layout = UiLayout::new(1920.0, 1080.0);
        let mut state = UiState::default();

        state.handle_key(KeyCode::Digit1, layout, None);
        assert_eq!(state.screen, Screen::Domains);
        state.activate(InteractionTarget::Back, Point::default(), layout);

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

        assert_eq!(state.screen, Screen::MainMenu);
    }
}
