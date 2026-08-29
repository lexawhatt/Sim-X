#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::presentation) enum Screen {
    Domains,
    PhysSubdomains,
    MechanicsEditor,
    TimeEasterEgg,
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
pub(in crate::presentation) enum PhysSubdomain {
    Mechanics,
    Thermodynamics,
    WavesAndOptics,
    Electromagnetism,
    Relativity,
    FluidDynamics,
    Sandbox,
}

impl PhysSubdomain {
    pub(in crate::presentation) const ALL: [Self; 7] = [
        Self::Mechanics,
        Self::Thermodynamics,
        Self::WavesAndOptics,
        Self::Electromagnetism,
        Self::Relativity,
        Self::FluidDynamics,
        Self::Sandbox,
    ];

    pub(in crate::presentation) const fn index(self) -> usize {
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

    pub(in crate::presentation) const fn label(self) -> &'static str {
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

    pub(in crate::presentation) const fn title(self) -> &'static str {
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
    use super::{PhysSubdomain, TopDomain};

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
}
