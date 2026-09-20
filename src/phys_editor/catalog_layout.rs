//! Shared geometry for the category dock and its bounded object browser.

use sim_logic::prelude::LogicalScreenPosition;

use super::{
    catalog::{CatalogState, CatalogTarget, Category, EntryId, PAGE_SIZE},
    layout::{Layout, Rect},
};

pub(crate) struct CatalogLayout {
    pub(crate) panel: Rect,
    pub(crate) search: Rect,
    pub(crate) grid: Rect,
    pub(crate) rail: Rect,
    pub(crate) footer: Rect,
    pub(crate) quick_count: usize,
    dock: Rect,
}

impl CatalogLayout {
    pub(crate) fn new(layout: &Layout, catalog: &CatalogState) -> Self {
        let dock = layout.palette;
        let width = dock.width.min(600.0);
        let height = (dock.y - 142.0).clamp(268.0, 460.0);
        let x = dock.x;
        let y = dock.y - height - 10.0 + 12.0 * (1.0 - catalog.blend);
        let panel = Rect::new(x, y, width, height);
        let rail_width = 118.0;
        let grid = Rect::new(
            x + 16.0,
            y + 104.0,
            width - rail_width - 40.0,
            height - 154.0,
        );
        Self {
            panel,
            dock,
            quick_count: if dock.width >= 900.0 { 8 } else { 6 },
            search: Rect::new(x + 16.0, y + 54.0, width - 32.0, 36.0),
            grid,
            rail: Rect::new(
                x + width - rail_width - 16.0,
                grid.y,
                rail_width,
                grid.height,
            ),
            footer: Rect::new(x + 16.0, y + height - 40.0, width - 32.0, 28.0),
        }
    }

    pub(crate) fn category(&self, category: Category) -> Rect {
        let height = ((self.rail.height - 16.0) / 5.0).clamp(19.0, 30.0);
        Rect::new(
            self.rail.x,
            self.rail.y + category.index() as f32 * (height + 4.0),
            self.rail.width,
            height,
        )
    }

    pub(crate) fn quick(&self, index: usize) -> Option<Rect> {
        if index >= self.quick_count {
            return None;
        }
        let width = ((self.dock.width - self.opener().width - 34.0) / self.quick_count as f32
            - 6.0)
            .min(110.0);
        Some(Rect::new(
            self.dock.x + 12.0 + index as f32 * (width + 6.0),
            self.dock.y + 36.0,
            width,
            96.0,
        ))
    }

    pub(crate) fn opener(&self) -> Rect {
        let width = if self.quick_count == 6 { 94.0 } else { 118.0 };
        Rect::new(
            self.dock.x + self.dock.width - width - 12.0,
            self.dock.y + 36.0,
            width,
            96.0,
        )
    }

    pub(crate) fn section(&self, index: usize) -> Option<Rect> {
        let y = self.rail.y + 186.0 + index as f32 * 30.0;
        (y + 30.0 <= self.rail.y + self.rail.height).then_some(Rect::new(
            self.rail.x,
            y,
            self.rail.width,
            30.0,
        ))
    }

    pub(crate) fn card(&self, index: usize) -> Option<Rect> {
        if index >= PAGE_SIZE {
            return None;
        }
        let width = (self.grid.width - 16.0) / 3.0;
        let height = (self.grid.height - 16.0) / 3.0;
        Some(Rect::new(
            self.grid.x + (index % 3) as f32 * (width + 8.0),
            self.grid.y + (index / 3) as f32 * (height + 8.0),
            width,
            height,
        ))
    }

    pub(crate) fn target(&self, target: CatalogTarget, catalog: &CatalogState) -> Option<Rect> {
        match target {
            CatalogTarget::Category(category) => catalog.open.then(|| self.category(category)),
            CatalogTarget::Quick(id) => self.quick(
                catalog
                    .quick_entries()
                    .iter()
                    .position(|entry| *entry == id)?,
            ),
            CatalogTarget::Open => Some(self.opener()),
            CatalogTarget::Favorite(id) => {
                let card = self.card(catalog.visible().iter().position(|entry| *entry == id)?)?;
                Some(Rect::new(
                    card.x + card.width - 23.0,
                    card.y + 2.0,
                    21.0,
                    card.height.min(23.0) - 2.0,
                ))
            }
            CatalogTarget::IdealFilter => EntryId::ALL
                .into_iter()
                .any(|id| id.entry().ideal)
                .then_some(Rect::new(
                    self.panel.x + self.panel.width - 150.0,
                    self.panel.y + 12.0,
                    92.0,
                    28.0,
                )),
            CatalogTarget::Search => Some(self.search),
            CatalogTarget::ClearSearch => Some(Rect::new(
                self.search.x + self.search.width - 34.0,
                self.search.y + 2.0,
                32.0,
                32.0,
            )),
            CatalogTarget::Close => Some(Rect::new(
                self.panel.x + self.panel.width - 46.0,
                self.panel.y + 10.0,
                30.0,
                30.0,
            )),
            CatalogTarget::Section(index) => {
                catalog.category.sections().get(index)?;
                self.section(index)
            }
            CatalogTarget::Entry(id) => {
                self.card(catalog.visible().iter().position(|entry| *entry == id)?)
            }
            CatalogTarget::PreviousPage => Some(Rect::new(
                self.footer.x + self.footer.width - 64.0,
                self.footer.y,
                28.0,
                28.0,
            )),
            CatalogTarget::NextPage => Some(Rect::new(
                self.footer.x + self.footer.width - 28.0,
                self.footer.y,
                28.0,
                28.0,
            )),
        }
    }

    pub(crate) fn hit(
        &self,
        point: LogicalScreenPosition,
        catalog: &CatalogState,
    ) -> Option<CatalogTarget> {
        if catalog.open {
            let targets = [
                CatalogTarget::Close,
                CatalogTarget::ClearSearch,
                CatalogTarget::Search,
                CatalogTarget::IdealFilter,
                CatalogTarget::PreviousPage,
                CatalogTarget::NextPage,
            ];
            for target in targets {
                if self
                    .target(target, catalog)
                    .is_some_and(|rect| rect.contains(point))
                {
                    return Some(target);
                }
            }
            for index in 0..catalog.category.sections().len() {
                if self.section(index).is_some_and(|rect| rect.contains(point)) {
                    return Some(CatalogTarget::Section(index));
                }
            }
            for (index, id) in catalog.visible().into_iter().enumerate() {
                if self
                    .target(CatalogTarget::Favorite(id), catalog)
                    .is_some_and(|rect| rect.contains(point))
                {
                    return Some(CatalogTarget::Favorite(id));
                }
                if self.card(index).is_some_and(|rect| rect.contains(point)) {
                    return Some(CatalogTarget::Entry(id));
                }
            }
            if let Some(category) = Category::ALL
                .into_iter()
                .find(|category| self.category(*category).contains(point))
            {
                return Some(CatalogTarget::Category(category));
            }
        }
        if self.opener().contains(point) {
            return Some(CatalogTarget::Open);
        }
        catalog
            .quick_entries()
            .into_iter()
            .enumerate()
            .find(|(index, _)| self.quick(*index).is_some_and(|rect| rect.contains(point)))
            .map(|(_, entry)| CatalogTarget::Quick(entry))
    }
}
