#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::presentation) enum Screen {
    MainMenu,
    Settings,
    Domains,
    PhysSubdomains,
    Projects,
    PhysicsEditor,
    PhysicsView,
    PhysicsViewExit,
    TimeEasterEgg,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::presentation) enum ViewExitChoice {
    Apply,
    Discard,
    Cancel,
}

impl ViewExitChoice {
    pub(in crate::presentation) const ALL: [Self; 3] = [Self::Apply, Self::Discard, Self::Cancel];

    pub(in crate::presentation) const fn index(self) -> usize {
        match self {
            Self::Apply => 0,
            Self::Discard => 1,
            Self::Cancel => 2,
        }
    }

    pub(in crate::presentation) const fn label(self) -> &'static str {
        match self {
            Self::Apply => "APPLY",
            Self::Discard => "DISCARD",
            Self::Cancel => "CANCEL",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::presentation) enum MainMenuItem {
    Domains,
    Settings,
    Quit,
}

impl MainMenuItem {
    pub(in crate::presentation) const ALL: [Self; 3] = [Self::Domains, Self::Settings, Self::Quit];

    pub(in crate::presentation) const fn index(self) -> usize {
        match self {
            Self::Domains => 0,
            Self::Settings => 1,
            Self::Quit => 2,
        }
    }

    pub(in crate::presentation) const fn label(self) -> &'static str {
        match self {
            Self::Domains => "DOMAINS",
            Self::Settings => "SETTINGS",
            Self::Quit => "QUIT",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::presentation) enum SocialLink {
    GitHub,
    YouTube,
    Telegram,
}

impl SocialLink {
    pub(in crate::presentation) const ALL: [Self; 3] =
        [Self::GitHub, Self::YouTube, Self::Telegram];

    pub(in crate::presentation) const fn index(self) -> usize {
        match self {
            Self::GitHub => 0,
            Self::YouTube => 1,
            Self::Telegram => 2,
        }
    }

    pub(in crate::presentation) const fn label(self) -> &'static str {
        match self {
            Self::GitHub => "GITHUB",
            Self::YouTube => "YOUTUBE",
            Self::Telegram => "TELEGRAM",
        }
    }

    pub(in crate::presentation) const fn url(self) -> &'static str {
        match self {
            Self::GitHub => "https://github.com/lexawhatt/Sim-X",
            Self::YouTube => "https://www.youtube.com/@LexaWhat",
            Self::Telegram => "https://t.me/Simulation_X",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::presentation) enum TopDomain {
    Math,
    Phys,
    Chem,
    Biol,
}

impl TopDomain {
    pub(in crate::presentation) const ALL: [Self; 4] =
        [Self::Math, Self::Phys, Self::Chem, Self::Biol];

    pub(in crate::presentation) const fn index(self) -> usize {
        match self {
            Self::Math => 0,
            Self::Phys => 1,
            Self::Chem => 2,
            Self::Biol => 3,
        }
    }

    pub(in crate::presentation) const fn label(self) -> &'static str {
        match self {
            Self::Math => "MATH",
            Self::Phys => "PHYS",
            Self::Chem => "CHEM",
            Self::Biol => "BIOL",
        }
    }

    pub(in crate::presentation) const fn title(self) -> &'static str {
        match self {
            Self::Math => "SIM;MATH",
            Self::Phys => "SIM;PHYS",
            Self::Chem => "SIM;CHEM",
            Self::Biol => "SIM;BIOL",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PhysSubdomain {
    Mechanics,
    Thermodynamics,
    WavesAndOptics,
    Electromagnetism,
    Relativity,
    FluidDynamics,
    Sandbox,
}

impl PhysSubdomain {
    pub(crate) const ALL: [Self; 7] = [
        Self::Mechanics,
        Self::Thermodynamics,
        Self::WavesAndOptics,
        Self::Electromagnetism,
        Self::Relativity,
        Self::FluidDynamics,
        Self::Sandbox,
    ];

    pub(crate) const fn index(self) -> usize {
        match self {
            Self::Mechanics => 0,
            Self::Thermodynamics => 1,
            Self::WavesAndOptics => 2,
            Self::Electromagnetism => 3,
            Self::Relativity => 4,
            Self::FluidDynamics => 5,
            Self::Sandbox => 6,
        }
    }

    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Mechanics => "MECHANICS",
            Self::Thermodynamics => "THERMODYNAMICS",
            Self::WavesAndOptics => "WAVES & OPTICS",
            Self::Electromagnetism => "ELECTROMAGNETISM",
            Self::Relativity => "RELATIVITY",
            Self::FluidDynamics => "FLUID DYNAMICS",
            Self::Sandbox => "SANDBOX",
        }
    }

    pub(crate) const fn title(self) -> &'static str {
        match self {
            Self::Mechanics => "PHYS;MECHANICS",
            Self::Thermodynamics => "PHYS;THERMODYNAMICS",
            Self::WavesAndOptics => "PHYS;WAVES & OPTICS",
            Self::Electromagnetism => "PHYS;ELECTROMAGNETISM",
            Self::Relativity => "PHYS;RELATIVITY",
            Self::FluidDynamics => "PHYS;FLUID DYNAMICS",
            Self::Sandbox => "PHYS;SANDBOX",
        }
    }

    pub(crate) const fn has_runtime(self) -> bool {
        matches!(
            self,
            Self::Mechanics | Self::Thermodynamics | Self::WavesAndOptics | Self::Electromagnetism
        )
    }

    /// Whether this first slice advances through playback controls in View.
    pub(crate) const fn supports_playback(self) -> bool {
        matches!(
            self,
            Self::Mechanics | Self::Thermodynamics | Self::WavesAndOptics
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ProjectTemplate {
    Blank,
    PrimaryLab,
    SecondaryLab,
}

impl ProjectTemplate {
    pub(crate) const ALL: [Self; 3] = [Self::Blank, Self::PrimaryLab, Self::SecondaryLab];

    pub(crate) const fn index(self) -> usize {
        match self {
            Self::Blank => 0,
            Self::PrimaryLab => 1,
            Self::SecondaryLab => 2,
        }
    }

    pub(crate) const fn label(self, subdomain: PhysSubdomain) -> &'static str {
        match (self, subdomain) {
            (Self::Blank, _) => "STARTER SCENE",
            (Self::PrimaryLab, PhysSubdomain::Mechanics) => "NEWTON II LAB",
            (Self::SecondaryLab, PhysSubdomain::Mechanics) => "PENDULUM DRAFT",
            (Self::PrimaryLab, PhysSubdomain::Thermodynamics) => "HEAT EXCHANGE",
            (Self::SecondaryLab, PhysSubdomain::Thermodynamics) => "THERMAL NETWORK",
            (Self::PrimaryLab, PhysSubdomain::WavesAndOptics) => "STRING PULSE",
            (Self::SecondaryLab, PhysSubdomain::WavesAndOptics) => "STANDING WAVE",
            (Self::PrimaryLab, PhysSubdomain::Electromagnetism) => "ELECTRIC DIPOLE",
            (Self::SecondaryLab, PhysSubdomain::Electromagnetism) => "FIELD MAPPING",
            (Self::PrimaryLab, _) => "RUNTIME PENDING",
            (Self::SecondaryLab, _) => "PROJECT PENDING",
        }
    }

    pub(crate) const fn description(self, subdomain: PhysSubdomain) -> &'static str {
        match (self, subdomain) {
            (Self::Blank, PhysSubdomain::Mechanics) => "DEFAULT EDITABLE SETUP",
            (Self::Blank, _) => "CURATED STARTER SETUP",
            (Self::PrimaryLab, PhysSubdomain::Mechanics) => "FORCE MASS ACCELERATION",
            (Self::SecondaryLab, PhysSubdomain::Mechanics) => "AWAITS CONSTRAINT GATE",
            (Self::PrimaryLab, PhysSubdomain::Thermodynamics) => "CONDUCTION TO EQUILIBRIUM",
            (Self::SecondaryLab, PhysSubdomain::Thermodynamics) => "MULTI BODY CONDUCTION",
            (Self::PrimaryLab, PhysSubdomain::WavesAndOptics) => "FIXED ENDPOINT WAVE",
            (Self::SecondaryLab, PhysSubdomain::WavesAndOptics) => "FIXED ENDPOINT MODE",
            (Self::PrimaryLab, PhysSubdomain::Electromagnetism) => "POINT CHARGE FIELD",
            (Self::SecondaryLab, PhysSubdomain::Electromagnetism) => "PROBE GRID MAP",
            (_, _) => "SCIENTIFIC RUNTIME NOT BUILT",
        }
    }

    pub(crate) const fn is_available(self, subdomain: PhysSubdomain) -> bool {
        match (self, subdomain) {
            (Self::Blank | Self::PrimaryLab, subdomain) => subdomain.has_runtime(),
            (
                Self::SecondaryLab,
                PhysSubdomain::Thermodynamics
                | PhysSubdomain::WavesAndOptics
                | PhysSubdomain::Electromagnetism,
            ) => true,
            (Self::SecondaryLab, _) => false,
        }
    }
}

/// User-facing mapping from wall time to fixed scientific steps.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PlaybackRate {
    Paused,
    Slow,
    Normal,
    Fast,
}

impl PlaybackRate {
    pub(crate) const ALL: [Self; 4] = [Self::Paused, Self::Slow, Self::Normal, Self::Fast];

    pub(crate) const fn index(self) -> usize {
        match self {
            Self::Paused => 0,
            Self::Slow => 1,
            Self::Normal => 2,
            Self::Fast => 3,
        }
    }

    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Paused => "PAUSE",
            Self::Slow => "0.25X",
            Self::Normal => "1X",
            Self::Fast => "4X",
        }
    }

    pub(crate) const fn multiplier(self) -> f64 {
        match self {
            Self::Paused => 0.0,
            Self::Slow => 0.25,
            Self::Normal => 1.0,
            Self::Fast => 4.0,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::presentation) enum MechanicsTool {
    Pendulum,
    Body,
    Spring,
    Force,
}

impl MechanicsTool {
    pub(in crate::presentation) const ALL: [Self; 4] =
        [Self::Pendulum, Self::Body, Self::Spring, Self::Force];

    pub(in crate::presentation) const fn index(self) -> usize {
        match self {
            Self::Pendulum => 0,
            Self::Body => 1,
            Self::Spring => 2,
            Self::Force => 3,
        }
    }

    pub(in crate::presentation) const fn label(self) -> &'static str {
        match self {
            Self::Pendulum => "PENDULUM",
            Self::Body => "BODY",
            Self::Spring => "SPRING",
            Self::Force => "FORCE",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{PhysSubdomain, ProjectTemplate, SocialLink, TopDomain};

    #[test]
    fn product_catalog_keeps_expected_domain_order() {
        assert_eq!(
            TopDomain::ALL,
            [
                TopDomain::Math,
                TopDomain::Phys,
                TopDomain::Chem,
                TopDomain::Biol,
            ]
        );
    }

    #[test]
    fn physics_catalog_has_seven_subdomains_and_ends_with_sandbox() {
        assert_eq!(PhysSubdomain::ALL.len(), 7);
        assert_eq!(PhysSubdomain::ALL[0], PhysSubdomain::Mechanics);
        assert_eq!(PhysSubdomain::ALL[6], PhysSubdomain::Sandbox);
    }

    #[test]
    fn social_links_match_the_official_project_urls() {
        assert_eq!(
            SocialLink::GitHub.url(),
            "https://github.com/lexawhatt/Sim-X"
        );
        assert_eq!(
            SocialLink::YouTube.url(),
            "https://www.youtube.com/@LexaWhat"
        );
        assert_eq!(SocialLink::Telegram.url(), "https://t.me/Simulation_X");
    }

    #[test]
    fn secondary_labs_are_enabled_only_when_their_science_exists() {
        assert!(!ProjectTemplate::SecondaryLab.is_available(PhysSubdomain::Mechanics));
        assert!(ProjectTemplate::SecondaryLab.is_available(PhysSubdomain::Thermodynamics));
        assert!(ProjectTemplate::SecondaryLab.is_available(PhysSubdomain::WavesAndOptics));
        assert!(ProjectTemplate::SecondaryLab.is_available(PhysSubdomain::Electromagnetism));
        assert!(!ProjectTemplate::SecondaryLab.is_available(PhysSubdomain::Relativity));
    }
}
