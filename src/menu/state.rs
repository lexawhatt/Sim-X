use sim_logic::prelude::*;

/// Fixed external destinations. No user-provided URL reaches the OS launcher.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SocialLink {
    Github,
    Youtube,
    Telegram,
}

impl SocialLink {
    pub(crate) const fn url(self) -> &'static str {
        match self {
            Self::Github => "https://github.com/lexawhatt/Sim-X",
            Self::Youtube => "https://www.youtube.com/@LexaWhat",
            Self::Telegram => "https://t.me/Simulation_X",
        }
    }

    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::Github => "GitHub",
            Self::Youtube => "YouTube",
            Self::Telegram => "Telegram",
        }
    }

    pub(crate) const fn index(self) -> usize {
        match self {
            Self::Github => 0,
            Self::Youtube => 1,
            Self::Telegram => 2,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Domain {
    Phys,
    Math,
    Chem,
    Biol,
}

impl Domain {
    pub(crate) const ALL: [Self; 4] = [Self::Phys, Self::Math, Self::Chem, Self::Biol];

    pub(crate) const fn index(self) -> usize {
        match self {
            Self::Phys => 0,
            Self::Math => 1,
            Self::Chem => 2,
            Self::Biol => 3,
        }
    }

    pub(crate) const fn title(self) -> &'static str {
        match self {
            Self::Phys => "Sim;Phys",
            Self::Math => "Sim;Math",
            Self::Chem => "Sim;Chem",
            Self::Biol => "Sim;Biol",
        }
    }

    pub(crate) const fn description(self) -> &'static str {
        match self {
            Self::Phys => "Motion. Energy. The rules of nature.",
            Self::Math => "Geometry, functions, and patterns.",
            Self::Chem => "Elements. Bonds. New combinations.",
            Self::Biol => "Life. Growth. Connected systems.",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Overlay {
    None,
    Domains,
    PhysicsScales,
    Projects,
    CreateProject,
    Settings,
    Quit,
}

/// Physics workspaces are explicit choices, never automatic zoom modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PhysicsScale {
    Micro,
    Macro,
    Astra,
}

impl PhysicsScale {
    pub(crate) const ALL: [Self; 3] = [Self::Micro, Self::Macro, Self::Astra];
    pub(crate) const fn index(self) -> usize {
        match self {
            Self::Micro => 0,
            Self::Macro => 1,
            Self::Astra => 2,
        }
    }
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Micro => "Micro",
            Self::Macro => "Macro",
            Self::Astra => "Astra",
        }
    }
    pub(crate) const fn description(self) -> &'static str {
        match self {
            Self::Micro => "The microscopic world. Workspace coming soon.",
            Self::Macro => "Build with bodies, springs, and real interactions.",
            Self::Astra => "Celestial scales. Workspace coming soon.",
        }
    }
}

pub(crate) const PROJECT_NAME_LIMIT: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Target {
    Domains,
    Settings,
    Quit,
    Social(SocialLink),
    ReduceMotion,
    Close,
    ConfirmQuit,
    Domain(Domain),
    BackToMenu,
    Scale(PhysicsScale),
    BackToDomains,
    BackToScales,
    CreateProject,
    ProjectName,
    ConfirmProject,
    CancelProject,
}

impl Target {
    pub(crate) const ALL: [Self; 23] = [
        Self::Domains,
        Self::Settings,
        Self::Quit,
        Self::Social(SocialLink::Github),
        Self::Social(SocialLink::Youtube),
        Self::Social(SocialLink::Telegram),
        Self::ReduceMotion,
        Self::Close,
        Self::ConfirmQuit,
        Self::Domain(Domain::Phys),
        Self::Domain(Domain::Math),
        Self::Domain(Domain::Chem),
        Self::Domain(Domain::Biol),
        Self::BackToMenu,
        Self::Scale(PhysicsScale::Micro),
        Self::Scale(PhysicsScale::Macro),
        Self::Scale(PhysicsScale::Astra),
        Self::BackToDomains,
        Self::BackToScales,
        Self::CreateProject,
        Self::ProjectName,
        Self::ConfirmProject,
        Self::CancelProject,
    ];

    pub(crate) const fn index(self) -> usize {
        match self {
            Self::Domains => 0,
            Self::Settings => 1,
            Self::Quit => 2,
            Self::Social(link) => 3 + link.index(),
            Self::ReduceMotion => 6,
            Self::Close => 7,
            Self::ConfirmQuit => 8,
            Self::Domain(domain) => 9 + domain.index(),
            Self::BackToMenu => 13,
            Self::Scale(scale) => 14 + scale.index(),
            Self::BackToDomains => 17,
            Self::BackToScales => 18,
            Self::CreateProject => 19,
            Self::ProjectName => 20,
            Self::ConfirmProject => 21,
            Self::CancelProject => 22,
        }
    }
}

#[derive(Resource)]
pub(crate) struct MenuState {
    pub(crate) overlay: Overlay,
    pub(crate) focus: KeyboardFocus<Target, Overlay>,
    pub(crate) shifts: [bool; 2],
    pub(crate) controls: [bool; 2],
    pub(crate) fullscreen_requested: bool,
    pub(crate) hovered: Option<Target>,
    pub(crate) pointer_navigation: bool,
    pub(crate) last_pointer: Option<PointerSample>,
    pub(crate) pointer: PointerButton<Target>,
    pub(crate) reduced_motion: bool,
    pub(crate) emphasis: [f32; Target::ALL.len()],
    pub(crate) phase: f32,
    pub(crate) home_seconds: f32,
    pub(crate) home_mix: [f32; 4],
    pub(crate) domains_blend: f32,
    /// Separate content layers: domains, scales, projects, then name form.
    pub(crate) picker_mix: [f32; 4],
    pub(crate) scale_blend: f32,
    pub(crate) scale_mix: [f32; 3],
    pub(crate) macro_seconds: f32,
    pub(crate) domain_mix: [f32; 4],
    pub(crate) preview_pulse: f32,
    pub(crate) last_preview_target: Option<Domain>,
    pub(crate) selected_domain: Option<Domain>,
    pub(crate) pending_link: Option<SocialLink>,
    pub(crate) project_name: String,
    pub(crate) project_error: &'static str,
    pub(crate) pending_project: Option<String>,
    pub(crate) pending_math: bool,
    pub(crate) link_status: &'static str,
    pub(crate) departing: bool,
}

impl Default for MenuState {
    fn default() -> Self {
        Self {
            overlay: Overlay::None,
            focus: KeyboardFocus::new(Target::ALL.len()),
            shifts: [false; 2],
            controls: [false; 2],
            fullscreen_requested: true,
            hovered: None,
            pointer_navigation: true,
            last_pointer: None,
            pointer: PointerButton::new(MouseButton::Left),
            reduced_motion: false,
            emphasis: [0.0; Target::ALL.len()],
            phase: 0.0,
            home_seconds: 0.0,
            home_mix: [1.0, 0.0, 0.0, 0.0],
            domains_blend: 0.0,
            picker_mix: [0.0; 4],
            scale_blend: 0.0,
            scale_mix: [0.0; 3],
            macro_seconds: 0.0,
            domain_mix: [0.0; 4],
            preview_pulse: 0.0,
            last_preview_target: None,
            selected_domain: None,
            pending_link: None,
            project_name: String::new(),
            project_error: "",
            pending_project: None,
            pending_math: false,
            link_status: "",
            departing: false,
        }
    }
}

impl MenuState {
    pub(crate) fn targets(&self) -> &'static [Target] {
        match self.overlay {
            Overlay::None => &Target::ALL[..6],
            Overlay::Domains => &[
                Target::Domain(Domain::Phys),
                Target::Domain(Domain::Math),
                Target::Domain(Domain::Chem),
                Target::Domain(Domain::Biol),
                Target::BackToMenu,
            ],
            Overlay::Settings => &[Target::ReduceMotion, Target::Close],
            Overlay::Quit => &[Target::Close, Target::ConfirmQuit],
            Overlay::PhysicsScales => &[
                Target::Scale(PhysicsScale::Micro),
                Target::Scale(PhysicsScale::Macro),
                Target::Scale(PhysicsScale::Astra),
                Target::BackToDomains,
            ],
            Overlay::Projects => &[Target::CreateProject, Target::BackToScales],
            Overlay::CreateProject => &[
                Target::ProjectName,
                Target::ConfirmProject,
                Target::CancelProject,
            ],
        }
    }

    pub(crate) fn show(&mut self, overlay: Overlay) -> LogicResult {
        self.pointer.cancel();
        self.overlay = overlay;
        self.project_error = "";
        // A stationary pointer from the previous screen must not preview a
        // newly exposed domain. Movement or another explicit action re-enables it.
        self.pointer_navigation = false;
        if overlay == Overlay::Domains {
            self.selected_domain = None;
        }
        let focused = match overlay {
            Overlay::None => None,
            Overlay::Settings => Some(Target::ReduceMotion),
            Overlay::Domains => None,
            Overlay::Quit => Some(Target::Close),
            Overlay::PhysicsScales => None,
            Overlay::Projects => Some(Target::CreateProject),
            Overlay::CreateProject => Some(Target::ProjectName),
        };
        self.set_focus(focused)?;
        self.hovered = None;
        // The picker return control keeps fading at its old position after
        // navigation; its highlight must decay rather than snap off at release.
        let return_emphasis = self.emphasis[Target::BackToMenu.index()];
        self.emphasis.fill(0.0);
        self.emphasis[Target::BackToMenu.index()] = return_emphasis;
        Ok(())
    }

    pub(crate) fn set_focus(&mut self, target: Option<Target>) -> LogicResult {
        self.focus
            .set_focused(self.overlay, self.targets(), target)?;
        Ok(())
    }

    pub(crate) fn navigate(&mut self, command: FocusCommand) -> LogicResult<Option<Target>> {
        self.pointer.cancel();
        Ok(self
            .focus
            .process(self.overlay, self.targets(), command)?
            .activated)
    }

    pub(crate) fn preview_domain(&self) -> Option<Domain> {
        if self.overlay != Overlay::Domains {
            return None;
        }
        match self.hovered.or(self.focus.focused()) {
            Some(Target::Domain(domain)) => Some(domain),
            _ => self.selected_domain,
        }
    }

    pub(crate) fn preview_scale(&self) -> Option<PhysicsScale> {
        match self.overlay {
            Overlay::PhysicsScales => match self.hovered.or(self.focus.focused()) {
                Some(Target::Scale(scale)) => Some(scale),
                _ => None,
            },
            Overlay::Projects | Overlay::CreateProject => Some(PhysicsScale::Macro),
            _ => None,
        }
    }

    pub(crate) fn exploration(&self) -> bool {
        matches!(
            self.overlay,
            Overlay::Domains | Overlay::PhysicsScales | Overlay::Projects | Overlay::CreateProject
        )
    }

    /// Appends bounded printable input; this does not claim native text/IME support.
    pub(crate) fn append_project_character(&mut self, character: char) {
        if !character.is_control() && self.project_name.chars().count() < PROJECT_NAME_LIMIT {
            self.project_name.push(character);
            self.project_error = "";
        }
    }

    /// Validates the name before asking the application to open a fresh editor.
    pub(crate) fn confirm_project(&mut self) -> bool {
        let name = self.project_name.trim();
        if name.is_empty() {
            self.project_error = "Give your project a name.";
            return false;
        }
        if name.chars().count() > PROJECT_NAME_LIMIT || name.chars().any(char::is_control) {
            self.project_error = "Use 1 to 64 printable characters.";
            return false;
        }
        self.pending_project = Some(name.to_owned());
        self.pointer.cancel();
        self.departing = true;
        true
    }
}
