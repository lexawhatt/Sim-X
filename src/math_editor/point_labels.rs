//! Screen-space labels prefer free space and retain a small readable backdrop.
use super::{
    assets::Fonts,
    drawing::*,
    layout::{Layout, Rect},
};

pub(super) struct PointLabels<'a> {
    fonts: &'a Fonts,
    clip: Rect,
    occupied: Vec<Rect>,
    lines: Vec<([f32; 2], [f32; 2])>,
}

impl<'a> PointLabels<'a> {
    pub(super) fn new(scene: &Scene, layout: &Layout, fonts: &'a Fonts) -> Self {
        let clip = layout.canvas;
        let mut occupied: Vec<_> = layout
            .controls
            .iter()
            .filter_map(|(_, r, _)| r.intersection(clip))
            .collect();
        occupied.extend(scene.texts.iter().filter_map(|t| {
            Rect::new(
                t.position[0] - 3.0,
                t.position[1] - 16.0,
                fonts.width(t.style, &t.text) + 6.0,
                22.0,
            )
            .intersection(clip)
        }));
        // Reserve overlays rendered after the construction labels as well.
        occupied.push(Rect::new(clip.x, clip.y, clip.w, 96.0));
        occupied.push(Rect::new(clip.x, clip.y + clip.h - 35.0, clip.w, 35.0));
        let lines = scene
            .lines
            .iter()
            .filter(|line| {
                line.depth >= 0.8
                    && line.color.alpha() > 0.1
                    && line.clip.intersection(clip).is_some()
            })
            .map(|line| (line.from, line.to))
            .collect();
        Self {
            fonts,
            clip,
            occupied,
            lines,
        }
    }

    pub(super) fn draw(&mut self, scene: &mut Scene, index: usize, point: [f32; 2]) {
        if !self.clip.contains(point) {
            return;
        }
        let label = format!("P{}", index + 1);
        let width = self.fonts.width(2, &label);
        let candidates = [
            [point[0] + 10.0, point[1] - 10.0],
            [point[0] + 10.0, point[1] + 24.0],
            [point[0] - width - 10.0, point[1] - 10.0],
            [point[0] - width - 10.0, point[1] + 24.0],
            [point[0] - width * 0.5, point[1] - 15.0],
            [point[0] - width * 0.5, point[1] + 31.0],
        ];
        let mut best = None;
        for (rank, position) in candidates.into_iter().enumerate() {
            let rect = Rect::new(position[0] - 3.0, position[1] - 16.0, width + 6.0, 21.0);
            if rect.x < self.clip.x
                || rect.y < self.clip.y
                || rect.x + rect.w > self.clip.x + self.clip.w
                || rect.y + rect.h > self.clip.y + self.clip.h
            {
                continue;
            }
            let blocked = self
                .occupied
                .iter()
                .filter(|r| r.intersection(rect).is_some())
                .count();
            let crossings = self
                .lines
                .iter()
                .filter(|(a, b)| clip_line(*a, *b, rect).is_some())
                .count();
            let score = blocked
                .saturating_mul(10_000)
                .saturating_add(crossings.saturating_mul(100))
                .saturating_add(rank);
            if best.as_ref().is_none_or(|(old, _, _)| score < *old) {
                best = Some((score, rect, position));
            }
        }
        if let Some((_, rect, position)) = best {
            self.occupied.push(rect);
            scene.panel(rect, BG.with_alpha(0.94), 4.0);
            scene.text(label, position, 2, INK, self.clip);
        }
    }
}
