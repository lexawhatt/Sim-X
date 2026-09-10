use super::{
    catalog::{
        MainMenuItem, MechanicsTool, PhysSubdomain, PlaybackRate, ProjectTemplate, Screen,
        SocialLink, TopDomain, ViewExitChoice,
    },
    geometry::{Point, UiRect},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::presentation) enum InteractionTarget {
    MainMenu(MainMenuItem),
    SocialLink(SocialLink),
    Domain(TopDomain),
    TimeCodeHotspot,
    PhysSubdomain(PhysSubdomain),
    Project(ProjectTemplate),
    Back,
    ViewSimulation,
    MechanicsTool(MechanicsTool),
    InspectorMassDecrease,
    InspectorMassIncrease,
    Playback(PlaybackRate),
    EditorCanvas,
    LabReset,
    ViewExitChoice(ViewExitChoice),
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

    pub(in crate::presentation) fn main_menu_buttons(self) -> [UiRect; 3] {
        let compact = self.height < 700.0;
        let width = (self.width - 80.0).clamp(280.0, 390.0);
        let height = if compact { 52.0 } else { 60.0 };
        let gap = if compact { 13.0 } else { 17.0 };
        let first_top = if compact { 230.0 } else { 290.0 };
        vertical_buttons(self.width, first_top, width, height, gap)
    }

    pub(in crate::presentation) fn social_link_buttons(self) -> [UiRect; 3] {
        let size = 42.0;
        let gap = 10.0;
        std::array::from_fn(|index| {
            UiRect::from_min_size(
                Point::new(
                    24.0 + index as f32 * (size + gap),
                    self.height - size - 22.0,
                ),
                size,
                size,
            )
        })
    }

    pub(in crate::presentation) fn domain_buttons(self) -> [UiRect; 4] {
        let compact = self.height < 700.0;
        let width = (self.width - 80.0).clamp(270.0, 360.0);
        let height = if compact { 52.0 } else { 60.0 };
        let gap = if compact { 13.0 } else { 17.0 };
        let first_top = if compact { 180.0 } else { 220.0 };
        vertical_buttons(self.width, first_top, width, height, gap)
    }

    pub(in crate::presentation) fn phys_buttons(self) -> [UiRect; 7] {
        let compact = self.height < 700.0;
        let width = (self.width - 96.0).clamp(360.0, 500.0);
        let height = if compact { 43.0 } else { 49.0 };
        let gap = if compact { 8.0 } else { 10.0 };
        let first_top = if compact { 138.0 } else { 164.0 };
        vertical_buttons(self.width, first_top, width, height, gap)
    }

    pub(in crate::presentation) fn project_cards(self) -> [UiRect; 3] {
        let compact = self.width < 1080.0;
        if compact {
            let width = (self.width - 80.0).clamp(340.0, 620.0);
            vertical_buttons(self.width, 190.0, width, 112.0, 18.0)
        } else {
            let gap = 20.0;
            let width = ((self.width - 120.0 - gap * 2.0) / 3.0).clamp(250.0, 360.0);
            let total = width * 3.0 + gap * 2.0;
            std::array::from_fn(|index| {
                UiRect::from_min_size(
                    Point::new(
                        (self.width - total) * 0.5 + index as f32 * (width + gap),
                        250.0,
                    ),
                    width,
                    180.0,
                )
            })
        }
    }

    pub(in crate::presentation) fn back_button(self) -> UiRect {
        UiRect::from_min_size(Point::new(24.0, 22.0), 160.0, 42.0)
    }

    pub(in crate::presentation) fn time_code_hotspot(self) -> UiRect {
        UiRect::from_min_size(
            Point::new(self.width - 58.0, self.height - 58.0),
            38.0,
            38.0,
        )
    }

    pub(in crate::presentation) fn lab_reset_button(self) -> UiRect {
        UiRect::from_min_size(Point::new(self.width - 140.0, 22.0), 116.0, 42.0)
    }

    pub(in crate::presentation) fn view_simulation_button(self) -> UiRect {
        UiRect::from_min_size(Point::new(self.width - 336.0, 22.0), 184.0, 42.0)
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

    pub(in crate::presentation) fn view_left_panel(self) -> UiRect {
        let width = (self.width * 0.16).clamp(156.0, 196.0);
        UiRect::from_min_size(Point::new(20.0, 84.0), width, self.height - 168.0)
    }

    pub(in crate::presentation) fn view_right_panel(self) -> UiRect {
        let width = (self.width * 0.20).clamp(180.0, 232.0);
        UiRect::from_min_size(
            Point::new(self.width - width - 20.0, 84.0),
            width,
            self.height - 168.0,
        )
    }

    pub(in crate::presentation) fn view_canvas(self) -> UiRect {
        let left = self.view_left_panel();
        let right = self.view_right_panel();
        UiRect {
            min: Point::new(left.max.x + 16.0, 84.0),
            max: Point::new(right.min.x - 16.0, self.height - 84.0),
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

    pub(in crate::presentation) fn inspector_mass_buttons(self) -> [UiRect; 2] {
        let panel = self.editor_right_panel();
        let gap = 10.0;
        let width = (panel.width() - 28.0 - gap) * 0.5;
        std::array::from_fn(|index| {
            UiRect::from_min_size(
                Point::new(
                    panel.min.x + 14.0 + index as f32 * (width + gap),
                    panel.min.y + 170.0,
                ),
                width,
                40.0,
            )
        })
    }

    pub(in crate::presentation) fn playback_buttons(self) -> [UiRect; 4] {
        let width = 94.0;
        let gap = 10.0;
        let total = width * 4.0 + gap * 3.0;
        std::array::from_fn(|index| {
            UiRect::from_min_size(
                Point::new(
                    (self.width - total) * 0.5 + index as f32 * (width + gap),
                    self.height - 66.0,
                ),
                width,
                42.0,
            )
        })
    }

    pub(in crate::presentation) fn view_exit_panel(self) -> UiRect {
        let width = (self.width - 64.0).clamp(440.0, 680.0);
        UiRect::from_min_size(
            Point::new((self.width - width) * 0.5, (self.height - 230.0) * 0.5),
            width,
            230.0,
        )
    }

    pub(in crate::presentation) fn view_exit_choice_buttons(self) -> [UiRect; 3] {
        let panel = self.view_exit_panel();
        let gap = 12.0;
        let width = (panel.width() - 48.0 - gap * 2.0) / 3.0;
        std::array::from_fn(|index| {
            UiRect::from_min_size(
                Point::new(
                    panel.min.x + 24.0 + index as f32 * (width + gap),
                    panel.max.y - 70.0,
                ),
                width,
                42.0,
            )
        })
    }

    pub(in crate::presentation) fn hit_test(
        self,
        screen: Screen,
        subdomain: PhysSubdomain,
        point: Point,
    ) -> Option<InteractionTarget> {
        match screen {
            Screen::MainMenu => {
                if self.time_code_hotspot().contains(point) {
                    return Some(InteractionTarget::TimeCodeHotspot);
                }
                if let Some(target) = SocialLink::ALL
                    .into_iter()
                    .zip(self.social_link_buttons())
                    .find_map(|(link, rect)| {
                        rect.contains(point)
                            .then_some(InteractionTarget::SocialLink(link))
                    })
                {
                    return Some(target);
                }
                MainMenuItem::ALL
                    .into_iter()
                    .zip(self.main_menu_buttons())
                    .find_map(|(item, rect)| {
                        rect.contains(point)
                            .then_some(InteractionTarget::MainMenu(item))
                    })
            }
            Screen::Settings => self
                .back_button()
                .contains(point)
                .then_some(InteractionTarget::Back),
            Screen::Domains => {
                if self.back_button().contains(point) {
                    return Some(InteractionTarget::Back);
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
                if self.back_button().contains(point) {
                    return Some(InteractionTarget::Back);
                }
                PhysSubdomain::ALL
                    .into_iter()
                    .zip(self.phys_buttons())
                    .find_map(|(subdomain, rect)| {
                        rect.contains(point)
                            .then_some(InteractionTarget::PhysSubdomain(subdomain))
                    })
            }
            Screen::Projects => {
                if self.back_button().contains(point) {
                    return Some(InteractionTarget::Back);
                }
                ProjectTemplate::ALL
                    .into_iter()
                    .zip(self.project_cards())
                    .find_map(|(project, rect)| {
                        rect.contains(point)
                            .then_some(InteractionTarget::Project(project))
                    })
            }
            Screen::PhysicsEditor => {
                if self.back_button().contains(point) {
                    return Some(InteractionTarget::Back);
                }
                if self.view_simulation_button().contains(point) {
                    return Some(InteractionTarget::ViewSimulation);
                }
                if self.lab_reset_button().contains(point) {
                    return Some(InteractionTarget::LabReset);
                }
                if subdomain == PhysSubdomain::Mechanics {
                    for (tool, rect) in MechanicsTool::ALL
                        .into_iter()
                        .zip(self.mechanics_tool_buttons())
                    {
                        if rect.contains(point) {
                            return Some(InteractionTarget::MechanicsTool(tool));
                        }
                    }
                    let [decrease, increase] = self.inspector_mass_buttons();
                    if decrease.contains(point) {
                        return Some(InteractionTarget::InspectorMassDecrease);
                    }
                    if increase.contains(point) {
                        return Some(InteractionTarget::InspectorMassIncrease);
                    }
                }
                self.editor_canvas()
                    .contains(point)
                    .then_some(InteractionTarget::EditorCanvas)
            }
            Screen::PhysicsView => {
                if self.back_button().contains(point) {
                    return Some(InteractionTarget::Back);
                }
                if self.lab_reset_button().contains(point) {
                    return Some(InteractionTarget::LabReset);
                }
                if subdomain != PhysSubdomain::Electromagnetism {
                    for (rate, rect) in PlaybackRate::ALL.into_iter().zip(self.playback_buttons()) {
                        if rect.contains(point) {
                            return Some(InteractionTarget::Playback(rate));
                        }
                    }
                }
                None
            }
            Screen::PhysicsViewExit => ViewExitChoice::ALL
                .into_iter()
                .zip(self.view_exit_choice_buttons())
                .find_map(|(choice, rect)| {
                    rect.contains(point)
                        .then_some(InteractionTarget::ViewExitChoice(choice))
                }),
            Screen::TimeEasterEgg => self
                .back_button()
                .contains(point)
                .then_some(InteractionTarget::Back),
        }
    }
}

fn vertical_buttons<const COUNT: usize>(
    screen_width: f32,
    first_top: f32,
    width: f32,
    height: f32,
    gap: f32,
) -> [UiRect; COUNT] {
    std::array::from_fn(|index| {
        UiRect::from_min_size(
            Point::new(
                (screen_width - width) * 0.5,
                first_top + index as f32 * (height + gap),
            ),
            width,
            height,
        )
    })
}

#[cfg(test)]
mod tests {
    use super::{InteractionTarget, UiLayout};
    use crate::presentation::ui::catalog::{
        MainMenuItem, MechanicsTool, PhysSubdomain, ProjectTemplate, Screen, SocialLink, TopDomain,
        ViewExitChoice,
    };

    #[test]
    fn main_menu_targets_buttons_links_and_time_hotspot() {
        let layout = UiLayout::new(1920.0, 1080.0);
        for (item, rect) in MainMenuItem::ALL
            .into_iter()
            .zip(layout.main_menu_buttons())
        {
            assert_eq!(
                layout.hit_test(Screen::MainMenu, PhysSubdomain::Mechanics, rect.center()),
                Some(InteractionTarget::MainMenu(item))
            );
        }
        for (link, rect) in SocialLink::ALL
            .into_iter()
            .zip(layout.social_link_buttons())
        {
            assert_eq!(
                layout.hit_test(Screen::MainMenu, PhysSubdomain::Mechanics, rect.center()),
                Some(InteractionTarget::SocialLink(link))
            );
        }
        assert_eq!(
            layout.hit_test(
                Screen::MainMenu,
                PhysSubdomain::Mechanics,
                layout.time_code_hotspot().center()
            ),
            Some(InteractionTarget::TimeCodeHotspot)
        );
    }

    #[test]
    fn domain_button_centers_hit_the_matching_domain() {
        let layout = UiLayout::new(1920.0, 1080.0);
        for (domain, rect) in TopDomain::ALL.into_iter().zip(layout.domain_buttons()) {
            assert_eq!(
                layout.hit_test(Screen::Domains, PhysSubdomain::Mechanics, rect.center()),
                Some(InteractionTarget::Domain(domain))
            );
        }
    }

    #[test]
    fn physics_button_centers_hit_all_seven_subdomains() {
        let layout = UiLayout::new(1920.0, 1080.0);
        for (subdomain, rect) in PhysSubdomain::ALL.into_iter().zip(layout.phys_buttons()) {
            assert_eq!(
                layout.hit_test(
                    Screen::PhysSubdomains,
                    PhysSubdomain::Mechanics,
                    rect.center()
                ),
                Some(InteractionTarget::PhysSubdomain(subdomain))
            );
        }
    }

    #[test]
    fn project_cards_are_a_separate_navigation_level() {
        let layout = UiLayout::new(1920.0, 1080.0);
        for (project, rect) in ProjectTemplate::ALL.into_iter().zip(layout.project_cards()) {
            assert_eq!(
                layout.hit_test(Screen::Projects, PhysSubdomain::Mechanics, rect.center()),
                Some(InteractionTarget::Project(project))
            );
        }
    }

    #[test]
    fn view_exit_confirmation_owns_its_choice_targets() {
        let layout = UiLayout::new(1920.0, 1080.0);
        for (choice, rect) in ViewExitChoice::ALL
            .into_iter()
            .zip(layout.view_exit_choice_buttons())
        {
            assert_eq!(
                layout.hit_test(
                    Screen::PhysicsViewExit,
                    PhysSubdomain::Mechanics,
                    rect.center()
                ),
                Some(InteractionTarget::ViewExitChoice(choice))
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
                layout.hit_test(
                    Screen::PhysicsEditor,
                    PhysSubdomain::Mechanics,
                    rect.center()
                ),
                Some(InteractionTarget::MechanicsTool(tool))
            );
        }
        assert_eq!(
            layout.hit_test(
                Screen::PhysicsEditor,
                PhysSubdomain::Mechanics,
                layout.editor_canvas().center()
            ),
            Some(InteractionTarget::EditorCanvas)
        );
        for (rate, rect) in crate::presentation::ui::catalog::PlaybackRate::ALL
            .into_iter()
            .zip(layout.playback_buttons())
        {
            assert_eq!(
                layout.hit_test(Screen::PhysicsView, PhysSubdomain::Mechanics, rect.center()),
                Some(InteractionTarget::Playback(rate))
            );
            assert_eq!(
                layout.hit_test(
                    Screen::PhysicsView,
                    PhysSubdomain::Electromagnetism,
                    rect.center()
                ),
                None
            );
        }
        assert_eq!(
            layout.hit_test(
                Screen::PhysicsEditor,
                PhysSubdomain::Mechanics,
                layout.view_simulation_button().center()
            ),
            Some(InteractionTarget::ViewSimulation)
        );
        assert_eq!(
            layout.hit_test(
                Screen::PhysicsEditor,
                PhysSubdomain::Mechanics,
                layout.playback_buttons()[2].center()
            ),
            Some(InteractionTarget::EditorCanvas)
        );
    }
}
