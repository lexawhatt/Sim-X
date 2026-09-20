//! Retained fast-access dock and searchable catalog; picking shares all geometry.

use super::{
    assets::EditorAssets,
    catalog::{CatalogState, CatalogTarget, Category, EntryId, PAGE_SIZE},
    catalog_icons::{CIRCLES_PER_ICON, LINES_PER_ICON, Symbol},
    catalog_layout::CatalogLayout,
    document::ObjectKind,
    layout::{Layout, Rect},
    placement::Placement,
    state::{EditorState, Mode, Tool},
};
use sim_logic::prelude::*;
use std::borrow::Cow;

const QUICK_SLOTS: usize = 8;
const SECTION_SLOTS: usize = 3;
const ACTIONS: [CatalogTarget; 6] = [
    CatalogTarget::Open,
    CatalogTarget::IdealFilter,
    CatalogTarget::ClearSearch,
    CatalogTarget::Close,
    CatalogTarget::PreviousPage,
    CatalogTarget::NextPage,
];
const ELEMENT_COUNT: usize =
    QUICK_SLOTS + PAGE_SIZE * 2 + Category::ALL.len() + SECTION_SLOTS + ACTIONS.len();
const ICON_COUNT: usize = QUICK_SLOTS * 2 + PAGE_SIZE * 2 + ACTIONS.len() + 1;
pub(crate) const RECT_COUNT: usize = 8 + ELEMENT_COUNT * 2;
pub(crate) const LINE_COUNT: usize = ICON_COUNT * LINES_PER_ICON;
pub(crate) const CIRCLE_COUNT: usize = ICON_COUNT * CIRCLES_PER_ICON;
pub(crate) const TEXT_COUNT: usize =
    11 + QUICK_SLOTS + PAGE_SIZE * 2 + Category::ALL.len() + SECTION_SLOTS + 2;
pub(crate) const ENTITY_COUNT: usize = RECT_COUNT + LINE_COUNT + CIRCLE_COUNT + TEXT_COUNT;

const INK: Color = Color::rgb(0.708376, 0.814847, 0.791298);
const SECONDARY: Color = Color::rgb(0.296138, 0.407240, 0.428690);
const QUIET: Color = Color::rgb(0.124772, 0.215861, 0.258183);
const ACCENT: Color = Color::rgb(0.191202, 0.723055, 0.485150);
const BORDER: Color = Color::rgb(0.029557, 0.059511, 0.076185);
const TILE: Color = Color::rgb(0.007499, 0.013702, 0.020289);
const PANEL: Color = Color::rgb(0.004777, 0.009134, 0.014444);

#[derive(Clone, Copy)]
enum Surface {
    Dock,
    Shadow,
    Border,
    Panel,
    SearchBorder,
    Search,
    FooterDivider,
    RailDivider,
}
const SURFACES: [Surface; 8] = [
    Surface::Dock,
    Surface::Shadow,
    Surface::Border,
    Surface::Panel,
    Surface::SearchBorder,
    Surface::Search,
    Surface::FooterDivider,
    Surface::RailDivider,
];
#[derive(Clone, Copy)]
enum Element {
    Quick(usize),
    Card(usize),
    Favorite(usize),
    Category(Category),
    Section(usize),
    Action(CatalogTarget),
}
#[derive(Clone, Copy)]
enum Label {
    DockTitle,
    DockHint,
    Title,
    Count,
    Search,
    EmptyTitle,
    EmptyDetail(usize),
    Footer(usize),
    Page,
    Quick(usize),
    Card(usize),
    CardCategory(usize),
    Category(Category),
    Section(usize),
    Open,
    Ideal,
}
const LABELS: [Label; 11] = [
    Label::DockTitle,
    Label::DockHint,
    Label::Title,
    Label::Count,
    Label::Search,
    Label::EmptyTitle,
    Label::EmptyDetail(0),
    Label::EmptyDetail(1),
    Label::Footer(0),
    Label::Footer(1),
    Label::Page,
];
#[derive(Clone, Copy)]
enum Icon {
    Quick(usize),
    QuickFavorite(usize),
    Card(usize),
    Favorite(usize),
    Action(CatalogTarget),
    Search,
}
#[derive(Component, Clone, Copy)]
pub(crate) struct CatalogVisualSlot(Slot);
#[derive(Clone, Copy)]
enum Slot {
    Surface(Surface),
    Element { element: Element, border: bool },
    Text(Label),
    Line { icon: usize, segment: usize },
    Circle { icon: usize, disc: usize },
}

fn elements() -> impl Iterator<Item = Element> {
    (0..QUICK_SLOTS)
        .map(Element::Quick)
        .chain((0..PAGE_SIZE).flat_map(|index| [Element::Card(index), Element::Favorite(index)]))
        .chain(Category::ALL.into_iter().map(Element::Category))
        .chain((0..SECTION_SLOTS).map(Element::Section))
        .chain(ACTIONS.into_iter().map(Element::Action))
}
fn icons() -> impl Iterator<Item = Icon> {
    (0..QUICK_SLOTS)
        .flat_map(|index| [Icon::Quick(index), Icon::QuickFavorite(index)])
        .chain((0..PAGE_SIZE).flat_map(|index| [Icon::Card(index), Icon::Favorite(index)]))
        .chain(ACTIONS.into_iter().map(Icon::Action))
        .chain([Icon::Search])
}
fn labels() -> impl Iterator<Item = Label> {
    LABELS
        .into_iter()
        .chain((0..QUICK_SLOTS).map(Label::Quick))
        .chain((0..PAGE_SIZE).flat_map(|index| [Label::Card(index), Label::CardCategory(index)]))
        .chain(Category::ALL.into_iter().map(Label::Category))
        .chain((0..SECTION_SLOTS).map(Label::Section))
        .chain([Label::Open, Label::Ideal])
}

pub(crate) fn spawn(world: &mut WorldBuilder, assets: &EditorAssets) -> LogicResult {
    let origin = LogicalScreenPosition::new(0.0, 0.0);
    let mut rectangle = ScreenRectangleVisual::new(
        origin,
        LogicalScreenVector::new(1.0, 1.0),
        Color::TRANSPARENT,
    )?;
    rectangle.set_clip(ScreenClip::Empty);
    for surface in SURFACES {
        world.spawn((CatalogVisualSlot(Slot::Surface(surface)), rectangle))?;
    }
    for element in elements() {
        for border in [true, false] {
            world.spawn((
                CatalogVisualSlot(Slot::Element { element, border }),
                rectangle,
            ))?;
        }
    }
    for label in labels() {
        let font = match label {
            Label::Title | Label::EmptyTitle => &assets.heading,
            Label::Search => &assets.body,
            _ => &assets.small,
        };
        let mut text = ScreenTextVisual::new(font.clone(), "", origin)?;
        text.set_clip(ScreenClip::Empty);
        world.spawn((CatalogVisualSlot(Slot::Text(label)), text))?;
    }
    let mut line = ScreenLineVisual::new(
        origin,
        LogicalScreenPosition::new(1.0, 0.0),
        1.0,
        Color::TRANSPARENT,
    )?;
    line.set_clip(ScreenClip::Empty);
    let mut circle = ScreenCircleVisual::new(origin, 1.0, Color::TRANSPARENT)?;
    circle.set_clip(ScreenClip::Empty);
    for icon in 0..ICON_COUNT {
        for segment in 0..LINES_PER_ICON {
            world.spawn((CatalogVisualSlot(Slot::Line { icon, segment }), line))?;
        }
        for disc in 0..CIRCLES_PER_ICON {
            world.spawn((CatalogVisualSlot(Slot::Circle { icon, disc }), circle))?;
        }
    }
    Ok(())
}

fn inset(rect: Rect, amount: f32) -> Rect {
    Rect::new(
        rect.x + amount,
        rect.y + amount,
        (rect.width - amount * 2.0).max(1.0),
        (rect.height - amount * 2.0).max(1.0),
    )
}
fn clip(rect: Rect) -> LogicResult<ScreenClip> {
    Ok(ScreenClip::new(rect.position(), rect.size())?)
}
fn fade(color: Color, blend: f32) -> Color {
    color.with_alpha(color.alpha() * blend)
}
fn highlighted(catalog: &CatalogState, target: CatalogTarget) -> bool {
    catalog.hovered == Some(target) || catalog.focused == Some(target)
}
fn entry_selected(id: EntryId, state: &EditorState) -> bool {
    match id {
        EntryId::Ball => {
            state.tool == Tool::Build && state.brush == Placement::Primitive(ObjectKind::Ball)
        }
        EntryId::Box => {
            state.tool == Tool::Build && state.brush == Placement::Primitive(ObjectKind::Box)
        }
        EntryId::Anchor => {
            state.tool == Tool::Build && state.brush == Placement::Primitive(ObjectKind::Anchor)
        }
        EntryId::Rod => state.tool == Tool::Rod,
        EntryId::Spring => state.tool == Tool::Spring,
        EntryId::Pendulum => state.tool == Tool::Build && state.brush == Placement::Pendulum,
        EntryId::Oscillator => state.tool == Tool::Build && state.brush == Placement::Oscillator,
        EntryId::BounceLab => state.tool == Tool::Build && state.brush == Placement::BounceLab,
    }
}

struct FrameCatalog<'a> {
    state: &'a EditorState,
    layout: &'a Layout,
    picker: CatalogLayout,
    entries: Vec<EntryId>,
    quick: Vec<EntryId>,
    pages: usize,
    count: usize,
}
impl<'a> FrameCatalog<'a> {
    fn new(state: &'a EditorState, layout: &'a Layout) -> Self {
        Self {
            state,
            layout,
            picker: CatalogLayout::new(layout, &state.catalog),
            entries: state.catalog.visible(),
            quick: state.catalog.quick_entries(),
            pages: state.catalog.page_count(),
            count: state.catalog.filtered().len(),
        }
    }
    fn target(&self, element: Element) -> Option<(CatalogTarget, Rect, bool)> {
        if matches!(element, Element::Action(CatalogTarget::IdealFilter))
            && !EntryId::ALL.into_iter().any(|id| id.entry().ideal)
        {
            return None;
        }
        let (target, rect, dock) = match element {
            Element::Quick(index) => (
                CatalogTarget::Quick(*self.quick.get(index)?),
                self.picker.quick(index)?,
                true,
            ),
            Element::Card(index) => (
                CatalogTarget::Entry(*self.entries.get(index)?),
                self.picker.card(index)?,
                false,
            ),
            Element::Favorite(index) => {
                let target = CatalogTarget::Favorite(*self.entries.get(index)?);
                (
                    target,
                    self.picker.target(target, &self.state.catalog)?,
                    false,
                )
            }
            Element::Category(category) => (
                CatalogTarget::Category(category),
                self.picker.category(category),
                false,
            ),
            Element::Section(index) => {
                self.state.catalog.category.sections().get(index)?;
                (
                    CatalogTarget::Section(index),
                    self.picker.section(index)?,
                    false,
                )
            }
            Element::Action(target) => (
                target,
                self.picker.target(target, &self.state.catalog)?,
                target == CatalogTarget::Open,
            ),
        };
        if !dock && self.state.catalog.blend <= 0.001 {
            return None;
        }
        if target == CatalogTarget::ClearSearch && self.state.catalog.query.is_empty() {
            return None;
        }
        Some((target, rect, dock))
    }
}

struct RectangleStyle {
    rect: Rect,
    color: Color,
    radius: f32,
    depth: f32,
}
fn surface_style(surface: Surface, frame: &FrameCatalog<'_>) -> Option<RectangleStyle> {
    let picker = &frame.picker;
    let catalog = &frame.state.catalog;
    let dock = matches!(surface, Surface::Dock);
    if !dock && catalog.blend <= 0.001 {
        return None;
    }
    let (rect, color, radius, depth) = match surface {
        Surface::Dock => (frame.layout.palette, Color::rgb8(12, 20, 27), 10.0, 3.2),
        Surface::Shadow => (
            Rect::new(
                picker.panel.x - 4.0,
                picker.panel.y + 5.0,
                picker.panel.width + 8.0,
                picker.panel.height + 5.0,
            ),
            Color::rgb8(1, 5, 8).with_alpha(0.7),
            14.0,
            6.8,
        ),
        Surface::Border => (picker.panel, Color::rgb8(61, 94, 98), 11.0, 7.0),
        Surface::Panel => (inset(picker.panel, 1.0), PANEL, 10.0, 7.1),
        Surface::SearchBorder => (
            picker.search,
            if catalog.search_focused {
                ACCENT
            } else {
                BORDER
            },
            6.0,
            8.0,
        ),
        Surface::Search => (inset(picker.search, 1.0), Color::rgb8(9, 16, 22), 5.0, 8.1),
        Surface::FooterDivider => (
            Rect::new(
                picker.footer.x,
                picker.footer.y - 8.0,
                picker.footer.width,
                1.0,
            ),
            BORDER,
            0.0,
            8.0,
        ),
        Surface::RailDivider => (
            Rect::new(picker.rail.x - 9.0, picker.rail.y, 1.0, picker.rail.height),
            BORDER,
            0.0,
            8.0,
        ),
    };
    Some(RectangleStyle {
        rect,
        color: if dock {
            color
        } else {
            fade(color, catalog.blend)
        },
        radius,
        depth,
    })
}
fn element_style(
    element: Element,
    border: bool,
    frame: &FrameCatalog<'_>,
) -> Option<RectangleStyle> {
    let (target, rect, dock) = frame.target(element)?;
    let catalog = &frame.state.catalog;
    let selected = match target {
        CatalogTarget::Quick(id) | CatalogTarget::Entry(id) => entry_selected(id, frame.state),
        CatalogTarget::Category(category) => {
            catalog.category == category && catalog.query.is_empty()
        }
        CatalogTarget::Section(index) => catalog.section == index,
        CatalogTarget::Open => catalog.open,
        CatalogTarget::IdealFilter => catalog.ideal_only,
        _ => false,
    };
    let hover = highlighted(catalog, target);
    let small = matches!(
        element,
        Element::Favorite(_)
            | Element::Action(
                CatalogTarget::ClearSearch
                    | CatalogTarget::Close
                    | CatalogTarget::PreviousPage
                    | CatalogTarget::NextPage
            )
    );
    let color = if let CatalogTarget::Category(category) = target {
        let weight = catalog.emphasis[category.index()]
            .clamp(0.0, 1.0)
            .max(if selected { 0.7 } else { 0.0 });
        if border {
            if selected {
                ACCENT.with_alpha(0.8)
            } else {
                Color::rgb8(
                    (48.0 + weight * 35.0) as u8,
                    (69.0 + weight * 62.0) as u8,
                    (78.0 + weight * 39.0) as u8,
                )
            }
        } else {
            Color::rgb8(
                (21.0 + weight * 8.0) as u8,
                (31.0 + weight * 18.0) as u8,
                (39.0 + weight * 14.0) as u8,
            )
        }
    } else if small {
        if hover && !border {
            Color::rgb8(35, 55, 61)
        } else {
            Color::TRANSPARENT
        }
    } else if border {
        if selected {
            ACCENT.with_alpha(0.8)
        } else if hover {
            ACCENT.with_alpha(0.45)
        } else {
            BORDER
        }
    } else if selected || hover {
        Color::rgb8(29, 49, 53)
    } else {
        TILE
    };
    Some(RectangleStyle {
        rect: if border { rect } else { inset(rect, 1.0) },
        color: if dock {
            color
        } else {
            fade(color, catalog.blend)
        },
        radius: if border { 6.0 } else { 5.0 },
        depth: if dock { 4.0 } else { 8.0 } + if border { 0.0 } else { 0.1 },
    })
}

#[derive(Clone, Copy)]
struct IconStyle {
    symbol: Symbol,
    center: LogicalScreenPosition,
    size: f32,
    color: Color,
    depth: f32,
    bounds: Rect,
}
fn entry_symbol(id: EntryId) -> Symbol {
    match id {
        EntryId::Ball => Symbol::Ball,
        EntryId::Box => Symbol::Box,
        EntryId::Anchor => Symbol::Anchor,
        EntryId::Rod => Symbol::Rod,
        EntryId::Spring => Symbol::Spring,
        EntryId::Pendulum => Symbol::Pendulum,
        EntryId::Oscillator => Symbol::Oscillator,
        EntryId::BounceLab => Symbol::BounceLab,
    }
}
fn icon_style(icon: Icon, frame: &FrameCatalog<'_>) -> Option<IconStyle> {
    let catalog = &frame.state.catalog;
    let (symbol, rect, center, size, color, dock) = match icon {
        Icon::Quick(index) | Icon::Card(index) => {
            let dock = matches!(icon, Icon::Quick(_));
            let (id, rect) = if dock {
                (*frame.quick.get(index)?, frame.picker.quick(index)?)
            } else {
                (*frame.entries.get(index)?, frame.picker.card(index)?)
            };
            if rect.height < 64.0 {
                return None;
            }
            let bottom = if dock { 30.0 } else { 35.0 };
            (
                entry_symbol(id),
                rect,
                LogicalScreenPosition::new(
                    rect.x + rect.width * 0.5,
                    rect.y + (rect.height - bottom) * 0.5,
                ),
                15.0_f32.min((rect.height - bottom - 8.0) * 0.5),
                ACCENT,
                dock,
            )
        }
        Icon::QuickFavorite(index) => {
            let id = frame.quick.get(index)?;
            if !catalog.favorites.contains(id) {
                return None;
            }
            let tile = frame.picker.quick(index)?;
            let rect = Rect::new(tile.x + tile.width - 18.0, tile.y + 4.0, 14.0, 14.0);
            (
                Symbol::Favorite,
                rect,
                LogicalScreenPosition::new(rect.x + 7.0, rect.y + 7.0),
                5.0,
                ACCENT,
                true,
            )
        }
        Icon::Favorite(index) => {
            let id = *frame.entries.get(index)?;
            let rect = frame.picker.target(CatalogTarget::Favorite(id), catalog)?;
            let color = if catalog.favorites.contains(&id) {
                ACCENT
            } else {
                QUIET
            };
            (
                Symbol::Favorite,
                rect,
                LogicalScreenPosition::new(rect.x + rect.width * 0.5, rect.y + rect.height * 0.5),
                6.5,
                color,
                false,
            )
        }
        Icon::Search => {
            let rect = Rect::new(
                frame.picker.search.x + 8.0,
                frame.picker.search.y + 8.0,
                20.0,
                20.0,
            );
            (
                Symbol::Search,
                rect,
                LogicalScreenPosition::new(rect.x + 10.0, rect.y + 10.0),
                8.0,
                QUIET,
                false,
            )
        }
        Icon::Action(target) => {
            if target == CatalogTarget::IdealFilter {
                return None;
            }
            let (_, rect, dock) = frame.target(Element::Action(target))?;
            let symbol = match target {
                CatalogTarget::Open => Symbol::Objects,
                CatalogTarget::PreviousPage => Symbol::Previous,
                CatalogTarget::NextPage => Symbol::Next,
                _ => Symbol::Close,
            };
            let enabled = match target {
                CatalogTarget::PreviousPage => catalog.page > 0,
                CatalogTarget::NextPage => catalog.page.saturating_add(1) < frame.pages,
                _ => true,
            };
            let y = if dock {
                rect.y + 27.0
            } else {
                rect.y + rect.height * 0.5
            };
            (
                symbol,
                rect,
                LogicalScreenPosition::new(rect.x + rect.width * 0.5, y),
                if dock { 14.0 } else { 8.0 },
                if enabled {
                    SECONDARY
                } else {
                    QUIET.with_alpha(0.35)
                },
                dock,
            )
        }
    };
    if !dock && catalog.blend <= 0.001 {
        return None;
    }
    Some(IconStyle {
        symbol,
        center,
        size,
        color: if dock {
            color
        } else {
            fade(color, catalog.blend)
        },
        depth: if dock { 5.0 } else { 9.0 },
        bounds: rect,
    })
}

struct TextStyle<'a> {
    text: Cow<'a, str>,
    bounds: Rect,
    color: Color,
    centered: bool,
    depth: f32,
}
fn short_title(id: EntryId) -> &'static str {
    match id {
        EntryId::Oscillator => "Oscillator",
        _ => id.entry().title,
    }
}
// These are bounded catalog explanations, not a replacement text layout engine.
// Native single-line clipping remains the final guard for unusual glyph widths.
fn two_lines(text: &str, width: f32) -> (&str, &str) {
    let limit = ((width / 7.2).floor() as usize).max(1);
    if text.len() <= limit {
        return (text, "");
    }
    let boundary = text
        .char_indices()
        .map(|(index, _)| index)
        .take_while(|index| *index <= limit)
        .last()
        .unwrap_or(0);
    let split = text[..boundary]
        .rfind(' ')
        .filter(|index| *index > 0)
        .unwrap_or(boundary);
    (&text[..split], text[split..].trim_start())
}
fn text_style<'a>(label: Label, frame: &'a FrameCatalog<'_>) -> Option<TextStyle<'a>> {
    let picker = &frame.picker;
    let catalog = &frame.state.catalog;
    let dock = matches!(
        label,
        Label::DockTitle | Label::DockHint | Label::Quick(_) | Label::Open
    );
    if !dock && catalog.blend <= 0.001 {
        return None;
    }
    let (text, bounds, color, centered): (Cow<'a, str>, _, _, _) = match label {
        Label::DockTitle => (
            "QUICK ACCESS".into(),
            Rect::new(
                frame.layout.palette.x + 14.0,
                frame.layout.palette.y + 8.0,
                125.0,
                22.0,
            ),
            SECONDARY,
            false,
        ),
        Label::DockHint => (
            "Favorites and recent objects".into(),
            Rect::new(
                frame.layout.palette.x + 152.0,
                frame.layout.palette.y + 8.0,
                frame.layout.palette.width - 170.0,
                22.0,
            ),
            QUIET,
            false,
        ),
        Label::Quick(index) | Label::Card(index) => {
            let quick = matches!(label, Label::Quick(_));
            let (id, rect) = if quick {
                (*frame.quick.get(index)?, picker.quick(index)?)
            } else {
                (*frame.entries.get(index)?, picker.card(index)?)
            };
            let compact = rect.height < 64.0;
            let y = if compact {
                rect.y
            } else if quick {
                rect.y + rect.height - 30.0
            } else {
                rect.y + rect.height - 36.0
            };
            let width = if compact && !quick {
                rect.width - 27.0
            } else {
                rect.width - 6.0
            };
            (
                if quick && rect.width < 80.0 && id == EntryId::BounceLab {
                    "Bounce".into()
                } else {
                    short_title(id).into()
                },
                Rect::new(
                    rect.x + 3.0,
                    y,
                    width.max(1.0),
                    if compact { rect.height } else { 22.0 },
                ),
                if !quick && !catalog.query.is_empty() {
                    ACCENT
                } else {
                    INK
                },
                true,
            )
        }
        Label::CardCategory(index) => {
            let id = *frame.entries.get(index)?;
            let rect = picker.card(index)?;
            if rect.height < 64.0 {
                return None;
            }
            let text = if id.entry().ideal {
                "IDEAL"
            } else {
                id.entry().category.label()
            };
            (
                text.into(),
                Rect::new(
                    rect.x + 3.0,
                    rect.y + rect.height - 18.0,
                    rect.width - 6.0,
                    17.0,
                ),
                QUIET,
                true,
            )
        }
        Label::Open => {
            let rect = picker.opener();
            (
                "Catalog  /".into(),
                Rect::new(
                    rect.x + 3.0,
                    rect.y + rect.height - 30.0,
                    rect.width - 6.0,
                    22.0,
                ),
                INK,
                true,
            )
        }
        Label::Ideal => {
            if !EntryId::ALL.into_iter().any(|id| id.entry().ideal) {
                return None;
            }
            let rect = picker.target(CatalogTarget::IdealFilter, catalog)?;
            (
                "Ideal only".into(),
                inset(rect, 4.0),
                if catalog.ideal_only {
                    ACCENT
                } else {
                    SECONDARY
                },
                true,
            )
        }
        Label::Title => (
            "Object catalog".into(),
            Rect::new(
                picker.panel.x + 16.0,
                picker.panel.y + 11.0,
                picker.panel.width - 270.0,
                30.0,
            ),
            INK,
            false,
        ),
        Label::Count => (
            format!("{} results", frame.count).into(),
            Rect::new(
                picker.panel.x + picker.panel.width - 250.0,
                picker.panel.y + 13.0,
                83.0,
                26.0,
            ),
            QUIET,
            true,
        ),
        Label::Search => {
            let text: Cow<'a, str> = if catalog.query.is_empty() {
                "Search objects or aliases (Latin input)".into()
            } else {
                catalog.query.as_str().into()
            };
            (
                text,
                Rect::new(
                    picker.search.x + 36.0,
                    picker.search.y + 2.0,
                    picker.search.width - 74.0,
                    32.0,
                ),
                if catalog.query.is_empty() { QUIET } else { INK },
                false,
            )
        }
        Label::Category(category) => {
            let rect = picker.category(category);
            (
                category.label().into(),
                Rect::new(rect.x + 7.0, rect.y, rect.width - 14.0, rect.height),
                if catalog.category == category && catalog.query.is_empty() {
                    ACCENT
                } else {
                    SECONDARY
                },
                false,
            )
        }
        Label::Section(index) => {
            let label = *catalog.category.sections().get(index)?;
            let rect = picker.section(index)?;
            (
                label.into(),
                Rect::new(rect.x + 7.0, rect.y, rect.width - 14.0, rect.height),
                if catalog.section == index {
                    ACCENT
                } else {
                    SECONDARY
                },
                false,
            )
        }
        Label::EmptyTitle => {
            if !frame.entries.is_empty() {
                return None;
            }
            let planned = catalog.query.is_empty()
                && matches!(catalog.category, Category::Measure | Category::Actuators);
            (
                if planned {
                    "Not available yet"
                } else {
                    "No matches"
                }
                .into(),
                Rect::new(
                    picker.grid.x + 8.0,
                    picker.grid.y + picker.grid.height * 0.5 - 42.0,
                    picker.grid.width - 16.0,
                    30.0,
                ),
                SECONDARY,
                true,
            )
        }
        Label::EmptyDetail(line) => {
            if !frame.entries.is_empty() {
                return None;
            }
            let width = picker.grid.width - 16.0;
            let message = if catalog.query.is_empty() {
                catalog.category.empty_message()
            } else {
                "Try another name or clear the search to browse categories."
            };
            let (first, second) = two_lines(message, width);
            (
                if line == 0 { first } else { second }.into(),
                Rect::new(
                    picker.grid.x + 8.0,
                    picker.grid.y + picker.grid.height * 0.5 - 6.0 + line as f32 * 19.0,
                    width,
                    19.0,
                ),
                QUIET,
                true,
            )
        }
        Label::Footer(line) => {
            let description = match catalog.hovered.or(catalog.focused) {
                Some(CatalogTarget::Entry(id) | CatalogTarget::Favorite(id)) => {
                    id.entry().description
                }
                _ => {
                    if catalog.query.is_empty() {
                        catalog.category.description()
                    } else {
                        "Search covers all categories. Star an object to keep it in quick access."
                    }
                }
            };
            let width = picker.footer.width - 122.0;
            let (first, second) = two_lines(description, width);
            (
                if line == 0 { first } else { second }.into(),
                Rect::new(
                    picker.footer.x,
                    picker.footer.y - 1.0 + line as f32 * 16.0,
                    width,
                    17.0,
                ),
                SECONDARY,
                false,
            )
        }
        Label::Page => (
            format!(
                "{}/{}",
                catalog.page.saturating_add(1).min(frame.pages),
                frame.pages
            )
            .into(),
            Rect::new(
                picker.footer.x + picker.footer.width - 118.0,
                picker.footer.y,
                45.0,
                picker.footer.height,
            ),
            QUIET,
            true,
        ),
    };
    Some(TextStyle {
        text,
        bounds,
        color: if dock {
            color
        } else {
            fade(color, catalog.blend)
        },
        centered,
        depth: if dock { 6.0 } else { 10.0 },
    })
}

fn catalog_visible(layout: &Layout, state: &EditorState) -> bool {
    layout.usable() && state.mode == Mode::Editor && !state.environment_open
}

pub(crate) fn refresh(
    viewport: FrameViewport,
    state: Option<Res<EditorState>>,
    mut rectangles: Query<(&CatalogVisualSlot, &mut ScreenRectangleVisual)>,
    mut texts: Query<(&CatalogVisualSlot, &mut ScreenTextVisual)>,
    mut lines: Query<(&CatalogVisualSlot, &mut ScreenLineVisual)>,
    mut circles: Query<(&CatalogVisualSlot, &mut ScreenCircleVisual)>,
) -> LogicResult {
    let Some(state) = state else {
        return Ok(());
    };
    let layout = Layout::for_state(viewport.logical(), &state);
    let frame = FrameCatalog::new(&state, &layout);
    let visible = catalog_visible(&layout, &state);
    let viewport_clip = clip(Rect::new(0.0, 0.0, layout.width, layout.height))?;
    let mut icon_styles = [None; ICON_COUNT];
    if visible {
        for (index, icon) in icons().enumerate() {
            icon_styles[index] = icon_style(icon, &frame);
        }
    }
    for (slot, mut visual) in &mut rectangles {
        visual.set_clip(ScreenClip::Empty);
        if !visible {
            continue;
        }
        let style = match slot.0 {
            Slot::Surface(surface) => surface_style(surface, &frame),
            Slot::Element { element, border } => element_style(element, border, &frame),
            _ => None,
        };
        let Some(style) = style else {
            continue;
        };
        visual.set_geometry(style.rect.position(), style.rect.size())?;
        visual.set_color(style.color)?;
        visual.set_corner_radius(style.radius)?;
        visual.set_draw_order_depth(style.depth)?;
        visual.set_clip(viewport_clip);
    }
    for (slot, mut visual) in &mut texts {
        visual.set_clip(ScreenClip::Empty);
        if !visible {
            continue;
        }
        let Slot::Text(label) = slot.0 else {
            continue;
        };
        let Some(style) = text_style(label, &frame) else {
            continue;
        };
        visual.set_text(&style.text)?;
        visual.set_alignment(if style.centered {
            TextAlignment::Center
        } else {
            TextAlignment::Left
        })?;
        let metrics = visual.metrics();
        visual.set_position(LogicalScreenPosition::new(
            style.bounds.x
                + if style.centered {
                    style.bounds.width * 0.5
                } else {
                    0.0
                },
            style.bounds.y + (style.bounds.height + metrics.ascent() + metrics.descent()) * 0.5,
        ))?;
        visual.set_tint(style.color)?;
        visual.set_draw_order_depth(style.depth)?;
        visual.set_clip(clip(style.bounds)?);
    }
    for (slot, mut visual) in &mut lines {
        visual.set_clip(ScreenClip::Empty);
        let Slot::Line { icon, segment } = slot.0 else {
            continue;
        };
        let Some(style) = icon_styles[icon] else {
            continue;
        };
        let Some(&(x1, y1, x2, y2)) = style.symbol.geometry().lines.get(segment) else {
            continue;
        };
        let center = style.center.to_vec2();
        *visual = ScreenLineVisual::new(
            LogicalScreenPosition::new(center.x() + x1 * style.size, center.y() + y1 * style.size),
            LogicalScreenPosition::new(center.x() + x2 * style.size, center.y() + y2 * style.size),
            1.6,
            style.color,
        )?;
        visual.set_draw_order_depth(style.depth)?;
        visual.set_clip(clip(style.bounds)?);
    }
    for (slot, mut visual) in &mut circles {
        visual.set_clip(ScreenClip::Empty);
        let Slot::Circle { icon, disc } = slot.0 else {
            continue;
        };
        let Some(style) = icon_styles[icon] else {
            continue;
        };
        let Some(&(x, y, radius)) = style.symbol.geometry().circles.get(disc) else {
            continue;
        };
        let center = style.center.to_vec2();
        visual.set_geometry(
            LogicalScreenPosition::new(center.x() + x * style.size, center.y() + y * style.size),
            radius * style.size,
        )?;
        visual.set_color(style.color)?;
        visual.set_draw_order_depth(style.depth)?;
        visual.set_clip(clip(style.bounds)?);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_quick_label_remains_readable_at_minimum_viewport() -> LogicResult {
        let mut app = Application::<super::super::input::EditorAction>::new(AppConfig::default())?;
        let assets = EditorAssets::register(&mut app)?;
        let mut state = EditorState::default();
        for id in EntryId::ALL {
            state.catalog.remember(id);
            let layout = Layout::for_state(LogicalViewport::new(900.0, 600.0)?, &state);
            let frame = FrameCatalog::new(&state, &layout);
            let style = text_style(Label::Quick(0), &frame).unwrap();
            let text =
                ScreenTextVisual::new(assets.small.clone(), &style.text, style.bounds.position())?;
            assert!(
                text.metrics().advance() <= style.bounds.width,
                "{} is clipped",
                style.text
            );
        }
        Ok(())
    }

    #[test]
    fn all_retained_pools_match_declared_budgets() {
        assert_eq!(elements().count(), ELEMENT_COUNT);
        assert_eq!(icons().count(), ICON_COUNT);
        assert_eq!(labels().count(), TEXT_COUNT);
        assert_eq!(SURFACES.len() + elements().count() * 2, RECT_COUNT);
        assert_eq!(
            ENTITY_COUNT,
            RECT_COUNT + LINE_COUNT + CIRCLE_COUNT + TEXT_COUNT
        );
        assert!(
            Category::ALL
                .into_iter()
                .all(|category| category.sections().len() <= SECTION_SLOTS)
        );
    }

    #[test]
    fn preview_environment_and_tiny_windows_cull_the_entire_catalog() -> LogicResult {
        let viewport = LogicalViewport::new(1280.0, 800.0)?;
        let mut state = EditorState::default();
        assert!(catalog_visible(
            &Layout::for_state(viewport, &state),
            &state
        ));
        state.environment_open = true;
        assert!(!catalog_visible(
            &Layout::for_state(viewport, &state),
            &state
        ));
        state.environment_open = false;
        state.mode = Mode::Preview;
        assert!(!catalog_visible(
            &Layout::for_state(viewport, &state),
            &state
        ));
        state.mode = Mode::Editor;
        assert!(!catalog_visible(
            &Layout::for_state(LogicalViewport::new(100.0, 100.0)?, &state),
            &state
        ));
        Ok(())
    }

    #[test]
    fn every_visible_section_has_its_own_rendered_label_and_button() -> LogicResult {
        let mut state = EditorState::default();
        state.catalog.open = true;
        state.catalog.blend = 1.0;
        for (width, height) in [(900.0, 600.0), (1280.0, 800.0), (1920.0, 1080.0)] {
            for category in Category::ALL {
                state.catalog.category = category;
                let layout = Layout::for_state(LogicalViewport::new(width, height)?, &state);
                let frame = FrameCatalog::new(&state, &layout);
                for index in 0..category.sections().len() {
                    let expected = frame.picker.section(index).is_some();
                    assert_eq!(
                        text_style(Label::Section(index), &frame).is_some(),
                        expected
                    );
                    assert_eq!(
                        element_style(Element::Section(index), false, &frame).is_some(),
                        expected
                    );
                }
            }
        }
        Ok(())
    }
    #[test]
    fn compact_cards_keep_labels_and_omit_crowded_icons() -> LogicResult {
        let mut state = EditorState::default();
        state.catalog.open = true;
        state.catalog.blend = 1.0;
        let layout = Layout::for_state(LogicalViewport::new(900.0, 600.0)?, &state);
        let frame = FrameCatalog::new(&state, &layout);
        assert!(icon_style(Icon::Card(0), &frame).is_none());
        let text = text_style(Label::Card(0), &frame).unwrap();
        assert_eq!(text.text, "Ball");
        assert!(text.bounds.height >= 20.0);
        Ok(())
    }
    #[test]
    fn closed_popup_is_culled_but_fast_access_remains() -> LogicResult {
        let state = EditorState::default();
        let layout = Layout::for_state(LogicalViewport::new(1280.0, 800.0)?, &state);
        let frame = FrameCatalog::new(&state, &layout);
        assert!(surface_style(Surface::Panel, &frame).is_none());
        assert!(text_style(Label::Title, &frame).is_none());
        assert!(icon_style(Icon::Card(0), &frame).is_none());
        assert!(icon_style(Icon::Quick(0), &frame).is_some());
        assert!(text_style(Label::Quick(0), &frame).is_some());
        Ok(())
    }

    #[test]
    fn unimplemented_actuators_and_unused_ideal_filter_are_not_offered_as_tools() -> LogicResult {
        let mut state = EditorState::default();
        state.catalog.open = true;
        state.catalog.blend = 1.0;
        state.catalog.category = Category::Actuators;
        let layout = Layout::for_state(LogicalViewport::new(1280.0, 800.0)?, &state);
        let frame = FrameCatalog::new(&state, &layout);
        assert!(frame.entries.is_empty());
        assert_eq!(
            text_style(Label::EmptyTitle, &frame).unwrap().text,
            "Not available yet"
        );
        assert!(text_style(Label::Ideal, &frame).is_none());
        assert!(
            frame
                .target(Element::Action(CatalogTarget::IdealFilter))
                .is_none()
        );
        assert!(icon_style(Icon::Action(CatalogTarget::IdealFilter), &frame).is_none());
        Ok(())
    }
    #[test]
    fn shared_font_shapes_catalog_metadata() -> LogicResult {
        let mut app = Application::<super::super::input::EditorAction>::new(AppConfig::default())?;
        let assets = EditorAssets::register(&mut app)?;
        for category in Category::ALL {
            for label in [
                category.label(),
                category.description(),
                category.empty_message(),
            ] {
                let text = ScreenTextVisual::new(
                    assets.small.clone(),
                    label,
                    LogicalScreenPosition::new(0.0, 0.0),
                )?;
                assert!(text.metrics().advance() > 0.0);
            }
        }
        for id in EntryId::ALL {
            ScreenTextVisual::new(
                assets.small.clone(),
                id.entry().description,
                LogicalScreenPosition::new(0.0, 0.0),
            )?;
            for alias in id.entry().aliases {
                ScreenTextVisual::new(
                    assets.small.clone(),
                    alias,
                    LogicalScreenPosition::new(0.0, 0.0),
                )?;
            }
        }
        Ok(())
    }
}
