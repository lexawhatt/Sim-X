use super::{
    catalog::{MechanicsTool, PhysSubdomain, Screen, TopDomain},
    geometry::{Point, UiRect},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::presentation) enum InteractionTarget {
    Domain(TopDomain),
    TimeCodeHotspot,
    PhysSubdomain(PhysSubdomain),
    BackToDomains,
    BackToPhys,
    MechanicsTool(MechanicsTool),
    EditorCanvas,
    EditorReset,
}

#[derive(Clone, Copy, Debug)]
pub(in crate::presentation) struct UiLayout {
    pub(in crate::presentation) width: f32,
    pub(in crate::presentation) height: f32,
}

impl UiLayout {
    pub(in crate::presentation) fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }

    pub(in crate::presentation) fn domain_buttons(self) -> [UiRect; 4] {
        let compact = self.height < 700.0;
        let button_width = (self.width - 80.0).clamp(270.0, 360.0);
        let button_height = if compact { 52.0 } else { 60.0 };
        let gap = if compact { 13.0 } else { 17.0 };
        let first_top = if compact { 180.0 } else { 220.0 };

        std::array::from_fn(|index| {
            UiRect::from_min_size(
                Point::new(
                    (self.width - button_width) * 0.5,
                    first_top + index as f32 * (button_height + gap),
                ),
                button_width,
                button_height,
            )
        })
    }

    pub(in crate::presentation) fn phys_buttons(self) -> [UiRect; 7] {
        let compact = self.height < 700.0;
        let button_width = (self.width - 96.0).clamp(360.0, 500.0);
        let button_height = if compact { 43.0 } else { 49.0 };
        let gap = if compact { 8.0 } else { 10.0 };
        let first_top = if compact { 138.0 } else { 164.0 };

        std::array::from_fn(|index| {
            UiRect::from_min_size(
                Point::new(
                    (self.width - button_width) * 0.5,
                    first_top + index as f32 * (button_height + gap),
                ),
                button_width,
                button_height,
            )
        })
    }

    pub(in crate::presentation) fn domains_back_button(self) -> UiRect {
        UiRect::from_min_size(Point::new(24.0, 22.0), 150.0, 42.0)
    }

    pub(in crate::presentation) fn time_code_hotspot(self) -> UiRect {
        UiRect::from_min_size(
            Point::new(self.width - 58.0, self.height - 58.0),
            38.0,
            38.0,
        )
    }

    pub(in crate::presentation) fn phys_back_button(self) -> UiRect {
        UiRect::from_min_size(Point::new(24.0, 22.0), 132.0, 42.0)
    }

    pub(in crate::presentation) fn editor_reset_button(self) -> UiRect {
        UiRect::from_min_size(Point::new(self.width - 140.0, 22.0), 116.0, 42.0)
    }

    pub(in crate::presentation) fn editor_left_panel(self) -> UiRect {
        let width = (self.width * 0.20).clamp(172.0, 224.0);
        UiRect::from_min_size(Point::new(20.0, 84.0), width, self.height - 104.0)
    }

    pub(in crate::presentation) fn editor_right_panel(self) -> UiRect {
        let width = (self.width * 0.22).clamp(188.0, 252.0);
        UiRect::from_min_size(
            Point::new(self.width - width - 20.0, 84.0),
            width,
            self.height - 104.0,
        )
    }

    pub(in crate::presentation) fn editor_canvas(self) -> UiRect {
        let left = self.editor_left_panel();
        let right = self.editor_right_panel();
        UiRect {
            min: Point::new(left.max.x + 16.0, 84.0),
            max: Point::new(right.min.x - 16.0, self.height - 20.0),
        }
    }

    pub(in crate::presentation) fn mechanics_tool_buttons(self) -> [UiRect; 4] {
        let panel = self.editor_left_panel();
        let button_width = panel.width() - 28.0;
        let button_height = 48.0;
        std::array::from_fn(|index| {
            UiRect::from_min_size(
                Point::new(panel.min.x + 14.0, panel.min.y + 72.0 + index as f32 * 60.0),
                button_width,
                button_height,
            )
        })
    }

    pub(in crate::presentation) fn hit_test(
        self,
        screen: Screen,
        point: Point,
    ) -> Option<InteractionTarget> {
        match screen {
            Screen::Domains => {
                if self.time_code_hotspot().contains(point) {
                    return Some(InteractionTarget::TimeCodeHotspot);
                }
                TopDomain::ALL
                    .into_iter()
                    .zip(self.domain_buttons())
                    .find_map(|(domain, rect)| {
                        rect.contains(point)
                            .then_some(InteractionTarget::Domain(domain))
                    })
            }
            Screen::PhysSubdomains => {
                if self.domains_back_button().contains(point) {
                    return Some(InteractionTarget::BackToDomains);
                }
                PhysSubdomain::ALL
                    .into_iter()
                    .zip(self.phys_buttons())
                    .find_map(|(subdomain, rect)| {
                        rect.contains(point)
                            .then_some(InteractionTarget::PhysSubdomain(subdomain))
                    })
            }
            Screen::MechanicsEditor => {
                if self.phys_back_button().contains(point) {
                    return Some(InteractionTarget::BackToPhys);
                }
                if self.editor_reset_button().contains(point) {
                    return Some(InteractionTarget::EditorReset);
                }
                for (tool, rect) in MechanicsTool::ALL
                    .into_iter()
                    .zip(self.mechanics_tool_buttons())
                {
                    if rect.contains(point) {
                        return Some(InteractionTarget::MechanicsTool(tool));
                    }
                }
                self.editor_canvas()
                    .contains(point)
                    .then_some(InteractionTarget::EditorCanvas)
            }
            Screen::TimeEasterEgg => self
                .domains_back_button()
                .contains(point)
                .then_some(InteractionTarget::BackToDomains),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{InteractionTarget, UiLayout};
    use crate::presentation::ui::catalog::{MechanicsTool, PhysSubdomain, Screen, TopDomain};

    #[test]
    fn domain_button_centers_hit_the_matching_domain() {
        let layout = UiLayout::new(1920.0, 1080.0);

        for (domain, rect) in TopDomain::ALL.into_iter().zip(layout.domain_buttons()) {
            assert_eq!(
                layout.hit_test(Screen::Domains, rect.center()),
                Some(InteractionTarget::Domain(domain))
            );
        }
    }

    #[test]
    fn main_menu_corner_hits_the_time_code_hotspot() {
        let layout = UiLayout::new(1920.0, 1080.0);

        assert_eq!(
            layout.hit_test(Screen::Domains, layout.time_code_hotspot().center()),
            Some(InteractionTarget::TimeCodeHotspot)
        );
    }

    #[test]
    fn physics_button_centers_hit_all_seven_subdomains() {
        let layout = UiLayout::new(1920.0, 1080.0);

        for (subdomain, rect) in PhysSubdomain::ALL.into_iter().zip(layout.phys_buttons()) {
            assert_eq!(
                layout.hit_test(Screen::PhysSubdomains, rect.center()),
                Some(InteractionTarget::PhysSubdomain(subdomain))
            );
        }
    }

    #[test]
    fn editor_separates_tools_from_canvas() {
        let layout = UiLayout::new(1920.0, 1080.0);

        for (tool, rect) in MechanicsTool::ALL
            .into_iter()
            .zip(layout.mechanics_tool_buttons())
        {
            assert_eq!(
                layout.hit_test(Screen::MechanicsEditor, rect.center()),
                Some(InteractionTarget::MechanicsTool(tool))
            );
        }
        assert_eq!(
            layout.hit_test(Screen::MechanicsEditor, layout.editor_canvas().center()),
            Some(InteractionTarget::EditorCanvas)
        );
    }
}
