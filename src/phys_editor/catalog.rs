//! Bounded picker metadata and transient navigation, independent of rendering.
//!
//! Catalog entries describe implemented authoring actions. Empty categories
//! explain future product areas without manufacturing executable placeholders.
//! This state is never part of a scene snapshot or its undo history.

use super::document::ObjectKind;

/// Maximum cards shown on one picker page.
pub(crate) const PAGE_SIZE: usize = 9;
/// Maximum UTF-8 bytes accepted by catalog search; input rejection is atomic.
pub(crate) const MAX_QUERY_BYTES: usize = 128;
/// Maximum favorite, recent and quick-dock entries, independently bounded.
pub(crate) const MAX_QUICK_ENTRIES: usize = 8;

/// Stable category-dock order; categories are not physics solver types.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum Category {
    Bodies,
    Connections,
    Assemblies,
    Measure,
    Actuators,
}

impl Category {
    /// Stable functional-category order, also used to group global results.
    pub(crate) const ALL: [Self; 5] = [
        Self::Bodies,
        Self::Connections,
        Self::Assemblies,
        Self::Measure,
        Self::Actuators,
    ];

    /// Position in the category dock; not a persisted scientific identifier.
    pub(crate) const fn index(self) -> usize {
        match self {
            Self::Bodies => 0,
            Self::Connections => 1,
            Self::Assemblies => 2,
            Self::Measure => 3,
            Self::Actuators => 4,
        }
    }

    /// Short dock and picker heading.
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Bodies => "Bodies",
            Self::Connections => "Connections",
            Self::Assemblies => "Assemblies",
            Self::Measure => "Measure",
            Self::Actuators => "Actuators",
        }
    }

    /// Explains the category's purpose without claiming unimplemented models.
    pub(crate) const fn description(self) -> &'static str {
        match self {
            Self::Bodies => "2D bodies with mass, rotation and frictionless collisions.",
            Self::Connections => "Relationships that connect existing physical bodies.",
            Self::Assemblies => {
                "Compositions of ordinary bodies and relationships; built-in recipes are available now."
            }
            Self::Measure => "Observation tools for reading and visualizing simulation quantities.",
            Self::Actuators => "Sources and controls that act on a physical system.",
        }
    }

    /// Section tabs within this category; zero always means the unfiltered All.
    ///
    /// A section is catalog organization, not a separate editor or runtime.
    pub(crate) const fn sections(self) -> &'static [&'static str] {
        match self {
            Self::Bodies | Self::Assemblies => &["All", "Mechanics"],
            Self::Connections => &["All", "Rigid", "Elastic"],
            Self::Measure => &["All"],
            Self::Actuators => &["All"],
        }
    }

    /// Honest empty-category or no-match message for the picker.
    pub(crate) const fn empty_message(self) -> &'static str {
        match self {
            Self::Measure => {
                "Measurement tools and teacher-facing data views are not implemented yet."
            }
            Self::Actuators => {
                "Actuator objects are not implemented yet. Scene constants belong in Environment."
            }
            _ => "No matching entries. Try another section or clear the search.",
        }
    }
}

/// Identity of an implemented catalog entry, separate from authored object IDs.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum EntryId {
    Ball,
    Box,
    Anchor,
    Rod,
    Spring,
    Pendulum,
    Oscillator,
    BounceLab,
}

impl EntryId {
    /// Stable catalog order, preserved when filtering and paginating.
    pub(crate) const ALL: [Self; 8] = [
        Self::Ball,
        Self::Box,
        Self::Anchor,
        Self::Rod,
        Self::Spring,
        Self::Pendulum,
        Self::Oscillator,
        Self::BounceLab,
    ];

    /// Returns immutable metadata for this implemented authoring action.
    pub(crate) const fn entry(self) -> &'static Entry {
        let index = match self {
            Self::Ball => 0,
            Self::Box => 1,
            Self::Anchor => 2,
            Self::Rod => 3,
            Self::Spring => 4,
            Self::Pendulum => 5,
            Self::Oscillator => 6,
            Self::BounceLab => 7,
        };
        &ENTRIES[index]
    }
}

/// Intent dispatched by the editor; choosing a card never runs physics here.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum EntryAction {
    Primitive(ObjectKind),
    Rod,
    Spring,
    Pendulum,
    Oscillator,
    BounceLab,
}

/// Static product metadata; canonical physical state stays in the document.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Entry {
    pub(crate) id: EntryId,
    pub(crate) title: &'static str,
    pub(crate) description: &'static str,
    pub(crate) keywords: &'static str,
    /// Search aliases, including localized names; not executable type names.
    pub(crate) aliases: &'static [&'static str],
    /// Whether this action explicitly controls an idealized source or field.
    pub(crate) ideal: bool,
    pub(crate) category: Category,
    /// Nonzero category section index; All includes every section.
    pub(crate) section: usize,
    pub(crate) action: EntryAction,
}

static ENTRIES: [Entry; 8] = [
    Entry {
        id: EntryId::Ball,
        title: "Ball",
        description: "Uniform 2D disk; mass, diameter, rotation, fixed state and restitution.",
        keywords: "mechanics mass body sphere circle particle collision bounce",
        aliases: &["шар", "сфера", "частица"],
        ideal: false,
        category: Category::Bodies,
        section: 1,
        action: EntryAction::Primitive(ObjectKind::Ball),
    },
    Entry {
        id: EntryId::Box,
        title: "Box",
        description: "Uniform 2D rectangle; width, height, rotation and corner attachments.",
        keywords: "mechanics mass body cube square rectangle platform wall collision",
        aliases: &[
            "ящик",
            "куб",
            "квадрат",
            "прямоугольник",
            "платформа",
            "стена",
        ],
        ideal: false,
        category: Category::Bodies,
        section: 1,
        action: EntryAction::Primitive(ObjectKind::Box),
    },
    Entry {
        id: EntryId::Anchor,
        title: "Anchor",
        description: "Fixed attachment point for a rod or spring; no collision shape.",
        keywords: "mechanics fixed support pivot attachment body",
        aliases: &["якорь", "опора", "крепление"],
        ideal: false,
        category: Category::Bodies,
        section: 1,
        action: EntryAction::Primitive(ObjectKind::Anchor),
    },
    Entry {
        id: EntryId::Rod,
        title: "Rod",
        description: "Keep two body centres a fixed distance apart in both directions.",
        keywords: "mechanics rigid distance constraint link connection",
        aliases: &["стержень", "связь", "штанга"],
        ideal: false,
        category: Category::Connections,
        section: 1,
        action: EntryAction::Rod,
    },
    Entry {
        id: EntryId::Spring,
        title: "Spring",
        description: "Attach surfaces or corners with a massless 20 N/m Hooke spring.",
        keywords: "mechanics elastic stiffness rest length link connection",
        aliases: &["пружина", "упругость"],
        ideal: false,
        category: Category::Connections,
        section: 2,
        action: EntryAction::Spring,
    },
    Entry {
        id: EntryId::Pendulum,
        title: "Pendulum",
        description: "Fixed anchor, 1 kg bob and 3 m rod; initially tilted 25 degrees.",
        keywords: "mechanics assembly prepared oscillation gravity swing",
        aliases: &["маятник", "колебания"],
        ideal: false,
        category: Category::Assemblies,
        section: 1,
        action: EntryAction::Pendulum,
    },
    Entry {
        id: EntryId::Oscillator,
        title: "Spring oscillator",
        description: "Fixed anchor and 1 kg bob; a 2 m spring initially extended to 2.5 m.",
        keywords: "mechanics assembly prepared oscillation spring pair hooke",
        aliases: &["пружинный маятник", "осциллятор", "колебания"],
        ideal: false,
        category: Category::Assemblies,
        section: 1,
        action: EntryAction::Oscillator,
    },
    Entry {
        id: EntryId::BounceLab,
        title: "Bounce lab",
        description: "Two falling spheres, restitution 0.2 / 0.8, above a fixed platform.",
        keywords: "mechanics assembly prepared bounce collision restitution impact experiment",
        aliases: &["отскок", "столкновение", "упругость удара"],
        ideal: false,
        category: Category::Assemblies,
        section: 1,
        action: EntryAction::BounceLab,
    },
];

/// Semantic pointer targets; the presentation layer owns their geometry.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum CatalogTarget {
    Open,
    Quick(EntryId),
    Favorite(EntryId),
    Category(Category),
    Section(usize),
    Entry(EntryId),
    Search,
    ClearSearch,
    IdealFilter,
    Close,
    PreviousPage,
    NextPage,
}

/// Transient picker state, intentionally excluded from authoring history.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct CatalogState {
    pub(crate) open: bool,
    pub(crate) category: Category,
    pub(crate) section: usize,
    pub(crate) query: String,
    pub(crate) ideal_only: bool,
    /// Most recently selected entries first; never part of authoring undo.
    pub(crate) recent: Vec<EntryId>,
    /// Stable user-selected order, capped without silently evicting favorites.
    pub(crate) favorites: Vec<EntryId>,
    pub(crate) search_focused: bool,
    pub(crate) page: usize,
    pub(crate) hovered: Option<CatalogTarget>,
    pub(crate) pressed: Option<CatalogTarget>,
    pub(crate) focused: Option<CatalogTarget>,
    /// Presentation-owned interpolation weight; never changes filter results.
    pub(crate) blend: f32,
    /// Category hover weights owned by presentation, in stable dock order.
    pub(crate) emphasis: [f32; Category::ALL.len()],
}

impl Default for CatalogState {
    fn default() -> Self {
        Self {
            open: false,
            category: Category::Bodies,
            section: 0,
            query: String::new(),
            ideal_only: false,
            recent: Vec::new(),
            favorites: Vec::new(),
            search_focused: false,
            page: 0,
            hovered: None,
            pressed: None,
            focused: None,
            blend: 0.0,
            emphasis: [0.0; Category::ALL.len()],
        }
    }
}

impl CatalogState {
    /// Opens the full catalog with search ready; preserves filters and query.
    pub(crate) fn open(&mut self) {
        self.open = true;
        self.search_focused = true;
        self.focused = Some(CatalogTarget::Search);
        self.cancel_capture();
    }

    /// Selects a category without closing the full catalog on a second click.
    pub(crate) fn select_category(&mut self, category: Category) {
        self.category = category;
        self.section = 0;
        self.query.clear();
        self.page = 0;
        self.open();
    }

    /// Closes the picker and clears its local focus/capture, preserving fade-out.
    pub(crate) fn close(&mut self) {
        self.open = false;
        self.search_focused = false;
        self.focused = None;
        self.cancel_capture();
    }

    /// Empty search browses a category/section. Nonempty search is global.
    ///
    /// Every Unicode-lowercased word must match metadata by substring or ordered
    /// subsequence (fuzzy abbreviation). Global results group by category, then
    /// relevance, then declaration order. This does not promise edit-distance
    /// correction, Unicode normalization, or locale-specific case folding.
    pub(crate) fn filtered(&self) -> Vec<EntryId> {
        let words: Vec<String> = self
            .query
            .split_whitespace()
            .map(str::to_lowercase)
            .collect();
        let mut matches: Vec<_> = EntryId::ALL
            .into_iter()
            .enumerate()
            .filter_map(|(index, id)| {
                let entry = id.entry();
                if (self.ideal_only && !entry.ideal)
                    || (words.is_empty()
                        && (entry.category != self.category
                            || (self.section != 0 && entry.section != self.section)))
                {
                    return None;
                }
                let score = words.iter().try_fold(0usize, |total, word| {
                    metadata_score(entry, word).map(|score| total + score)
                })?;
                Some((entry.category.index(), score, index, id))
            })
            .collect();
        matches.sort_by_key(|(category, score, index, _)| (*category, *score, *index));
        matches.into_iter().map(|(_, _, _, id)| id).collect()
    }

    /// Returns at most nine entries, clamping a stale page before indexing.
    pub(crate) fn visible(&self) -> Vec<EntryId> {
        let entries = self.filtered();
        let page = self.page.min(pages_for(entries.len()) - 1);
        entries
            .into_iter()
            .skip(page * PAGE_SIZE)
            .take(PAGE_SIZE)
            .collect()
    }

    /// Number of pages, with one empty page when there are no matching entries.
    pub(crate) fn page_count(&self) -> usize {
        pages_for(self.filtered().len())
    }

    /// Selects a valid section and resets paging/capture; false means no change.
    pub(crate) fn set_section(&mut self, section: usize) -> bool {
        if section >= self.category.sections().len() || section == self.section {
            return false;
        }
        self.section = section;
        self.page = 0;
        self.search_focused = false;
        self.cancel_capture();
        true
    }

    /// Sets a UTF-8 query atomically; controls and more than 128 bytes reject.
    ///
    /// Returns whether input was accepted (including an identical query).
    pub(crate) fn set_query(&mut self, query: &str) -> bool {
        if query.len() > MAX_QUERY_BYTES || query.chars().any(char::is_control) {
            return false;
        }
        self.query.clear();
        self.query.push_str(query);
        self.page = 0;
        self.cancel_capture();
        true
    }

    /// Appends one printable ASCII character or space, up to 128 query bytes.
    ///
    /// Rejected input preserves the full picker state and returns false.
    pub(crate) fn append_ascii(&mut self, character: char) -> bool {
        if !(character.is_ascii_graphic() || character == ' ')
            || self.query.len() >= MAX_QUERY_BYTES
        {
            return false;
        }
        let mut query = self.query.clone();
        query.push(character);
        self.set_query(&query)
    }

    /// Removes the last search character and resets paging when text changed.
    pub(crate) fn backspace(&mut self) -> bool {
        if self.query.pop().is_none() {
            return false;
        }
        self.page = 0;
        self.cancel_capture();
        true
    }

    /// Clears search and always resets paging; returns whether text changed.
    pub(crate) fn clear_query(&mut self) -> bool {
        let changed = !self.query.is_empty();
        self.query.clear();
        self.page = 0;
        self.cancel_capture();
        changed
    }

    /// Remembers a chosen action once, moving it to the front of bounded recents.
    pub(crate) fn remember(&mut self, entry: EntryId) {
        self.recent.retain(|candidate| *candidate != entry);
        self.recent.insert(0, entry);
        self.recent.truncate(MAX_QUICK_ENTRIES);
    }

    /// Toggles a favorite; returns its final membership, false also when full.
    ///
    /// Adding a ninth favorite does not evict an existing user choice.
    pub(crate) fn toggle_favorite(&mut self, entry: EntryId) -> bool {
        if let Some(index) = self
            .favorites
            .iter()
            .position(|candidate| *candidate == entry)
        {
            self.favorites.remove(index);
            return false;
        }
        if self.favorites.len() >= MAX_QUICK_ENTRIES {
            return false;
        }
        self.favorites.push(entry);
        true
    }

    /// Favorites first, then recents, then starter tools; deduplicated to eight.
    pub(crate) fn quick_entries(&self) -> Vec<EntryId> {
        let mut entries = Vec::with_capacity(MAX_QUICK_ENTRIES);
        for entry in self
            .favorites
            .iter()
            .chain(&self.recent)
            .chain(&STARTER_ENTRIES)
        {
            if !entries.contains(entry) {
                entries.push(*entry);
                if entries.len() == MAX_QUICK_ENTRIES {
                    break;
                }
            }
        }
        entries
    }

    fn cancel_capture(&mut self) {
        self.hovered = None;
        self.pressed = None;
    }
}

fn pages_for(count: usize) -> usize {
    count.div_ceil(PAGE_SIZE).max(1)
}

const STARTER_ENTRIES: [EntryId; 8] = [
    EntryId::Ball,
    EntryId::Box,
    EntryId::Anchor,
    EntryId::Rod,
    EntryId::Spring,
    EntryId::Pendulum,
    EntryId::Oscillator,
    EntryId::BounceLab,
];

fn metadata_score(entry: &Entry, word: &str) -> Option<usize> {
    [entry.title, entry.description, entry.keywords]
        .into_iter()
        .chain(entry.aliases.iter().copied())
        .filter_map(|field| fuzzy_score(&field.to_lowercase(), word))
        .min()
}

fn fuzzy_score(field: &str, word: &str) -> Option<usize> {
    if field == word {
        return Some(0);
    }
    if let Some(offset) = field.find(word) {
        return Some(1 + offset);
    }
    // Match abbreviations inside individual words, not across an entire
    // description: unrelated scattered letters must not produce false hits.
    field
        .split(|character: char| !character.is_alphanumeric())
        .filter_map(|token| subsequence_score(token, word))
        .min()
}

fn subsequence_score(field: &str, word: &str) -> Option<usize> {
    // Contiguous metadata matches always rank ahead of these fuzzy matches.
    let mut wanted = word.chars();
    let mut next = wanted.next()?;
    let mut first = None;
    let mut count = 0;
    for (index, character) in field.chars().enumerate() {
        if character != next {
            continue;
        }
        first.get_or_insert(index);
        count += 1;
        if let Some(character) = wanted.next() {
            next = character;
        } else {
            return Some(1024 + index + 1 - first.unwrap_or(0) - count);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn search(state: &mut CatalogState, text: &str) {
        assert!(state.set_query(text));
    }

    #[test]
    fn categories_and_entries_have_stable_valid_metadata() {
        for (index, category) in Category::ALL.into_iter().enumerate() {
            assert_eq!(category.index(), index);
            assert!(!category.label().is_empty());
            assert!(!category.description().is_empty());
            assert!(!category.empty_message().is_empty());
            assert_eq!(category.sections()[0], "All");
        }
        for id in EntryId::ALL {
            let entry = id.entry();
            assert_eq!(entry.id, id);
            assert!(!entry.title.is_empty());
            assert!(!entry.description.is_empty());
            assert!(!entry.keywords.is_empty());
            assert!(!entry.aliases.is_empty());
            assert!((1..entry.category.sections().len()).contains(&entry.section));
        }
    }

    #[test]
    fn stable_results_never_mix_categories_or_treat_assemblies_as_primitives() {
        let mut state = CatalogState::default();
        assert_eq!(
            state.filtered(),
            vec![EntryId::Ball, EntryId::Box, EntryId::Anchor]
        );
        state.select_category(Category::Assemblies);
        assert_eq!(
            state.filtered(),
            vec![EntryId::Pendulum, EntryId::Oscillator, EntryId::BounceLab]
        );
        state.select_category(Category::Connections);
        assert_eq!(state.filtered(), vec![EntryId::Rod, EntryId::Spring]);
        state.select_category(Category::Actuators);
        assert!(state.filtered().is_empty());
    }

    #[test]
    fn search_is_case_insensitive_and_matches_all_words_across_metadata_fields() {
        let mut state = CatalogState::default();
        search(&mut state, "BaLl MASS");
        assert_eq!(state.filtered(), vec![EntryId::Ball]);
        search(&mut state, "sphere");
        assert_eq!(state.filtered(), vec![EntryId::Ball, EntryId::BounceLab]);
        search(&mut state, "not-a-real-entry");
        assert!(state.filtered().is_empty());
        search(&mut state, "  ");
        assert_eq!(state.filtered().len(), 3);
    }

    #[test]
    fn active_search_is_global_ignoring_section_and_category() {
        let mut state = CatalogState::default();
        state.select_category(Category::Connections);
        assert!(state.set_section(1));
        assert_eq!(state.filtered(), vec![EntryId::Rod]);
        search(&mut state, "spring");
        assert!(state.filtered().contains(&EntryId::Spring));
        assert!(state.filtered().contains(&EntryId::Oscillator));
        assert!(state.set_section(2));
        state.clear_query();
        assert_eq!(state.filtered(), vec![EntryId::Spring]);
        state.page = 2;
        state.pressed = Some(CatalogTarget::Entry(EntryId::Spring));
        let before = state.clone();
        assert!(!state.set_section(999));
        assert_eq!(state, before);
        assert!(!state.set_section(2));
        assert_eq!(state, before);
        assert!(state.set_section(0));
        assert_eq!(state.page, 0);
        assert!(state.pressed.is_none());
    }

    #[test]
    fn opening_another_category_resets_filters_and_capture_without_resetting_fade() {
        let mut state = CatalogState::default();
        state.select_category(Category::Connections);
        state.set_section(2);
        search(&mut state, "spring");
        state.page = 4;
        state.blend = 0.75;
        state.search_focused = true;
        state.hovered = Some(CatalogTarget::NextPage);
        state.pressed = Some(CatalogTarget::Close);
        state.select_category(Category::Assemblies);
        assert!(state.open);
        assert_eq!(state.category, Category::Assemblies);
        assert_eq!(state.section, 0);
        assert_eq!(state.page, 0);
        assert!(state.query.is_empty());
        assert!(state.search_focused);
        assert_eq!(state.focused, Some(CatalogTarget::Search));
        assert!(state.hovered.is_none() && state.pressed.is_none());
        assert_eq!(state.blend, 0.75);
        state.close();
        assert!(!state.open);
        assert_eq!(state.blend, 0.75);
    }

    #[test]
    fn closing_drops_local_capture_and_focus_but_allows_a_fade_out() {
        let mut state = CatalogState::default();
        state.select_category(Category::Bodies);
        search(&mut state, "ball");
        state.search_focused = true;
        state.pressed = Some(CatalogTarget::Entry(EntryId::Ball));
        state.hovered = Some(CatalogTarget::Search);
        state.blend = 1.0;
        state.close();
        assert!(!state.open && !state.search_focused);
        assert!(state.hovered.is_none() && state.pressed.is_none());
        assert_eq!(state.blend, 1.0);
        assert_eq!(state.query, "ball");
        state.select_category(Category::Bodies);
        assert!(state.query.is_empty());
    }

    #[test]
    fn search_input_rejects_controls_unicode_and_over_budget_text_atomically() {
        let mut state = CatalogState::default();
        for invalid in ['\n', '\r', '\t', '\0', '\u{7f}', 'я', '²'] {
            let before = state.clone();
            assert!(!state.append_ascii(invalid));
            assert_eq!(state, before);
        }
        for _ in 0..MAX_QUERY_BYTES {
            assert!(state.append_ascii('x'));
        }
        let before = state.clone();
        assert_eq!(state.query.len(), MAX_QUERY_BYTES);
        assert!(!state.append_ascii('y'));
        assert_eq!(state, before);
        assert!(state.backspace());
        assert!(state.append_ascii(' '));
        assert_eq!(state.query.len(), MAX_QUERY_BYTES);
    }

    #[test]
    fn query_edits_reset_stale_page_and_pressed_entry() {
        let mut state = CatalogState {
            page: usize::MAX,
            pressed: Some(CatalogTarget::Entry(EntryId::Box)),
            ..CatalogState::default()
        };
        assert!(state.append_ascii('b'));
        assert_eq!(state.page, 0);
        assert!(state.pressed.is_none());
        state.page = 2;
        assert!(state.backspace());
        assert_eq!(state.page, 0);
        assert!(!state.backspace());
        state.page = 4;
        assert!(!state.clear_query());
        assert_eq!(state.page, 0);
    }

    #[test]
    fn measure_category_has_no_fake_actions_and_explicit_future_message() {
        let mut state = CatalogState::default();
        state.select_category(Category::Measure);
        assert!(state.filtered().is_empty());
        assert!(state.visible().is_empty());
        assert_eq!(state.page_count(), 1);
        assert!(
            Category::Measure
                .empty_message()
                .contains("not implemented yet")
        );
    }

    #[test]
    fn pagination_is_bounded_and_stale_pages_cannot_overflow() {
        assert_eq!(pages_for(0), 1);
        assert_eq!(pages_for(PAGE_SIZE), 1);
        assert_eq!(pages_for(PAGE_SIZE + 1), 2);
        assert_eq!(pages_for(PAGE_SIZE * 2), 2);
        assert!(pages_for(usize::MAX) > 1);
        let state = CatalogState {
            page: usize::MAX,
            ..CatalogState::default()
        };
        assert_eq!(state.visible(), state.filtered());
        assert!(state.visible().len() <= PAGE_SIZE);
        assert_eq!(state.page_count(), 1);
    }

    #[test]
    fn unicode_aliases_and_fuzzy_abbreviations_search_every_category() {
        let mut state = CatalogState::default();
        state.select_category(Category::Measure);
        for (query, expected) in [
            ("ШАР", EntryId::Ball),
            ("ящик", EntryId::Box),
            ("якорь", EntryId::Anchor),
            ("стержень", EntryId::Rod),
            ("пружина", EntryId::Spring),
            ("маятник", EntryId::Pendulum),
            ("pndlm", EntryId::Pendulum),
        ] {
            search(&mut state, query);
            assert!(state.filtered().contains(&expected), "{query}");
        }
        search(&mut state, "spring");
        let results = state.filtered();
        assert!(results.contains(&EntryId::Spring));
        assert!(results.contains(&EntryId::Oscillator));
        assert!(
            results.windows(2).all(|pair| {
                pair[0].entry().category.index() <= pair[1].entry().category.index()
            })
        );
        assert_eq!(fuzzy_score("pendulum", "pendulum"), Some(0));
        assert!(
            fuzzy_score("pendulum", "pend").unwrap() < fuzzy_score("pendulum", "pdlm").unwrap()
        );
        assert_eq!(fuzzy_score("pendulum", "zzzz"), None);
    }

    #[test]
    fn unicode_query_budget_and_backspace_preserve_utf8_and_reject_atomically() {
        let mut state = CatalogState::default();
        assert!(state.set_query(&"я".repeat(MAX_QUERY_BYTES / 2)));
        assert_eq!(state.query.len(), MAX_QUERY_BYTES);
        let before = state.clone();
        assert!(!state.set_query(&"я".repeat(MAX_QUERY_BYTES / 2 + 1)));
        assert_eq!(state, before);
        for invalid in ["new\nline", "tab\t", "delete\u{7f}"] {
            assert!(!state.set_query(invalid));
            assert_eq!(state, before);
        }
        assert!(!state.append_ascii('x'));
        assert!(state.backspace());
        assert_eq!(state.query.len(), MAX_QUERY_BYTES - 2);
        assert!(state.append_ascii('x'));
        assert!(state.set_query("маятник"));
        assert!(state.backspace());
        assert_eq!(state.query, "маятни");
    }

    #[test]
    fn ideal_filter_is_orthogonal_to_categories_and_global_query() {
        let mut state = CatalogState {
            ideal_only: true,
            ..CatalogState::default()
        };
        assert!(state.filtered().is_empty());
        state.select_category(Category::Actuators);
        assert!(state.filtered().is_empty());
        search(&mut state, "gravity");
        assert!(state.filtered().is_empty());
        assert!(state.filtered().iter().all(|id| id.entry().ideal));
        state.ideal_only = false;
        assert!(state.filtered().contains(&EntryId::Pendulum));
        assert!(!EntryId::Pendulum.entry().ideal);
    }

    #[test]
    fn quick_dock_prioritizes_favorites_then_recents_and_never_duplicates() {
        let mut state = CatalogState::default();
        assert_eq!(state.quick_entries(), STARTER_ENTRIES);
        state.remember(EntryId::Box);
        state.remember(EntryId::Rod);
        state.remember(EntryId::Box);
        assert_eq!(state.recent, vec![EntryId::Box, EntryId::Rod]);
        assert!(state.toggle_favorite(EntryId::Pendulum));
        assert!(state.toggle_favorite(EntryId::Rod));
        let quick = state.quick_entries();
        assert_eq!(
            &quick[..3],
            &[EntryId::Pendulum, EntryId::Rod, EntryId::Box]
        );
        assert_eq!(quick.len(), EntryId::ALL.len());
        assert!(quick.len() <= MAX_QUICK_ENTRIES);
        for (index, id) in quick.iter().enumerate() {
            assert!(!quick[..index].contains(id));
        }
        assert!(!state.toggle_favorite(EntryId::Rod));
        assert_eq!(state.favorites, vec![EntryId::Pendulum]);
    }

    #[test]
    fn recents_and_favorites_are_bounded_without_evicting_saved_favorites() {
        let mut state = CatalogState::default();
        for id in EntryId::ALL {
            state.remember(id);
            state.toggle_favorite(id);
        }
        assert_eq!(state.recent.len(), EntryId::ALL.len());
        assert!(state.recent.len() <= MAX_QUICK_ENTRIES);
        assert_eq!(state.recent[0], EntryId::BounceLab);
        assert!(state.recent.contains(&EntryId::Ball));
        assert_eq!(state.favorites, STARTER_ENTRIES);
        let before = state.favorites.clone();
        state.remember(EntryId::Ball);
        assert_eq!(state.recent[0], EntryId::Ball);
        assert_eq!(state.recent.len(), EntryId::ALL.len());
        state.close();
        state.select_category(Category::Assemblies);
        assert_eq!(state.favorites, before);
        assert_eq!(state.recent.len(), EntryId::ALL.len());
    }

    #[test]
    fn category_selection_stays_open_and_full_catalog_open_preserves_search() {
        let mut state = CatalogState::default();
        state.select_category(Category::Bodies);
        state.select_category(Category::Bodies);
        assert!(state.open && state.search_focused);
        search(&mut state, "маятник");
        state.close();
        state.open();
        assert!(state.open && state.search_focused);
        assert_eq!(state.query, "маятник");
        assert_eq!(state.focused, Some(CatalogTarget::Search));
    }
}
