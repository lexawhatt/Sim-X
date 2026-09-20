use sim_logic::prelude::*;

use super::state::{MenuState, Overlay, Target};

pub(crate) const BUTTON_RADIUS: f32 = 10.0;

#[derive(Debug, Clone, Copy)]
pub(crate) struct Rect {
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) width: f32,
    pub(crate) height: f32,
}

impl Rect {
    pub(crate) const fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub(crate) fn position(self) -> LogicalScreenPosition {
        LogicalScreenPosition::new(self.x, self.y)
    }

    pub(crate) fn size(self) -> LogicalScreenVector {
        LogicalScreenVector::new(self.width, self.height)
    }

    fn contains(self, point: LogicalScreenPosition) -> bool {
        let point = point.to_vec2();
        point.x() >= self.x
            && point.x() < self.x + self.width
            && point.y() >= self.y
            && point.y() < self.y + self.height
    }
}

pub(crate) struct Layout {
    pub(crate) width: f32,
    pub(crate) height: f32,
    pub(crate) left: f32,
    pub(crate) top: f32,
    pub(crate) menu_width: f32,
    pub(crate) compact: bool,
    pub(crate) short: bool,
    pub(crate) footer: f32,
    pub(crate) modal: Rect,
}

impl Layout {
    pub(crate) fn usable(&self) -> bool {
        self.width >= 360.0 && self.height >= 560.0
    }

    pub(crate) fn new(viewport: LogicalViewport) -> Self {
        let width = viewport.width();
        let height = viewport.height();
        let compact = width < 900.0;
        let menu_width = (width - 48.0).clamp(1.0, 352.0);
        let left = if compact {
            (width - menu_width) * 0.5
        } else {
            width * 0.12
        };
        let short = height < 680.0;
        let top = if short {
            76.0
        } else {
            ((height - 510.0) * 0.5).max(90.0)
        };
        let modal_width = (width - 32.0).clamp(1.0, 520.0);
        Self {
            width,
            height,
            left,
            top,
            menu_width,
            compact,
            short,
            footer: (height - 64.0).max(480.0),
            modal: Rect::new(
                (width - modal_width) * 0.5,
                (height - 316.0) * 0.5,
                modal_width,
                316.0,
            ),
        }
    }

    pub(crate) fn target(&self, target: Target, overlay: Overlay) -> Rect {
        match target {
            Target::Domains | Target::Settings | Target::Quit => Rect::new(
                self.left,
                self.top + if self.short { 190.0 } else { 240.0 } + target.index() as f32 * 64.0,
                self.menu_width,
                54.0,
            ),
            Target::Social(link) => {
                Rect::new(28.0 + link.index() as f32 * 52.0, self.footer, 42.0, 42.0)
            }
            Target::ReduceMotion => Rect::new(
                self.modal.x + 24.0,
                self.modal.y + 130.0,
                (self.modal.width - 48.0).max(1.0),
                54.0,
            ),
            Target::Close => {
                let width = if overlay == Overlay::Quit {
                    (self.modal.width - 60.0) * 0.5
                } else {
                    self.modal.width - 48.0
                };
                Rect::new(
                    self.modal.x + 24.0,
                    self.modal.y + 238.0,
                    width.max(1.0),
                    50.0,
                )
            }
            // This retained control keeps its picker geometry while fading out,
            // independent of the newly active screen or a modal's close button.
            Target::BackToMenu => Rect::new(
                (self.width - self.menu_width) * 0.5,
                self.domain_top() + 274.0,
                self.menu_width,
                42.0,
            ),
            Target::ConfirmQuit => Rect::new(
                self.modal.x + self.modal.width * 0.5 + 6.0,
                self.modal.y + 238.0,
                ((self.modal.width - 60.0) * 0.5).max(1.0),
                50.0,
            ),
            Target::Domain(domain) => Rect::new(
                (self.width - self.menu_width) * 0.5,
                self.domain_top() + domain.index() as f32 * 62.0,
                self.menu_width,
                52.0,
            ),
            Target::Scale(scale) => Rect::new(
                (self.width - self.menu_width) * 0.5,
                self.domain_top() + scale.index() as f32 * 72.0,
                self.menu_width,
                60.0,
            ),
            Target::BackToDomains => Rect::new(
                (self.width - self.menu_width) * 0.5,
                self.domain_top() + 238.0,
                self.menu_width,
                42.0,
            ),
            Target::CreateProject => Rect::new(
                (self.width - self.menu_width) * 0.5,
                self.domain_top() + 92.0,
                self.menu_width,
                56.0,
            ),
            Target::BackToScales => Rect::new(
                (self.width - self.menu_width) * 0.5,
                self.domain_top() + 174.0,
                self.menu_width,
                42.0,
            ),
            Target::ProjectName => Rect::new(
                self.modal.x + 24.0,
                self.modal.y + 122.0,
                (self.modal.width - 48.0).max(1.0),
                54.0,
            ),
            Target::CancelProject | Target::ConfirmProject => Rect::new(
                self.modal.x
                    + 24.0
                    + if target == Target::ConfirmProject {
                        (self.modal.width - 12.0) * 0.5
                    } else {
                        0.0
                    },
                self.modal.y + 238.0,
                ((self.modal.width - 60.0) * 0.5).max(1.0),
                50.0,
            ),
        }
    }

    pub(crate) fn domain_top(&self) -> f32 {
        ((self.height - 280.0) * 0.5).max(170.0)
    }

    pub(crate) fn backdrop(&self, blend: f32) -> Rect {
        let home = self.illustration();
        let size = (self.width * 0.85).min(self.height * 0.92).max(1.0);
        let centered = Rect::new(
            (self.width - size) * 0.5,
            (self.height - size) * 0.5,
            size,
            size,
        );
        Rect::new(
            home.x + (centered.x - home.x) * blend,
            home.y + (centered.y - home.y) * blend,
            home.width + (centered.width - home.width) * blend,
            home.height + (centered.height - home.height) * blend,
        )
    }

    pub(crate) fn illustration(&self) -> Rect {
        let size = (self.width * 0.43).min(self.height * 0.75).max(1.0);
        Rect::new(
            self.width * 0.72 - size * 0.5,
            self.height * 0.47 - size * 0.5,
            size,
            size,
        )
    }
}

pub(crate) fn hit(state: &MenuState, sample: Option<PointerSample>) -> Option<Target> {
    let sample = sample?;
    if !Rect::new(
        0.0,
        0.0,
        sample.viewport().width(),
        sample.viewport().height(),
    )
    .contains(sample.position())
    {
        return None;
    }
    // Queued edges retain their own viewport, including edges preceding a resize.
    let layout = Layout::new(sample.viewport());
    if !layout.usable() {
        return None;
    }
    state.targets().iter().copied().find(|&target| {
        let rect = layout.target(target, state.overlay);
        // Use Logic's rounded-fill picking, including event-time viewport bounds.
        ScreenRectangleVisual::rounded(
            rect.position(),
            rect.size(),
            Color::TRANSPARENT,
            BUTTON_RADIUS,
        )
        .is_ok_and(|visual| visual.contains_pointer(sample))
    })
}
