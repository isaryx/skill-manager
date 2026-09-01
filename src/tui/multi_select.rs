//! Full-screen, filterable multi-select list.
//!
//! Two modes: **list** (navigate and toggle) and **search** (type to filter). The list is
//! keyed by stable item keys, so filtering never disturbs the selection.

use dialoguer::console::{measure_text_width, style, truncate_str, Key, Term};

use super::{sanitize, Screen};
use crate::color::color_stderr;
use crate::error::SkmError;

/// `"> "` cursor gutter plus `"[x] "` checkbox. An item row is `ITEM_GUTTER + content`, so a
/// terminal narrower than this cannot be drawn without wrapping. No real terminal is.
const ITEM_GUTTER: usize = 6;
const SEPARATOR: &str = " · ";

/// One row in a [`MultiSelect`].
#[derive(Clone, Debug)]
pub struct MultiSelectItem {
    /// Returned verbatim on confirm. Never drawn — see `display`.
    key: String,
    /// `key` made safe to draw. Sanitizing once here keeps it off the render path.
    display: String,
    note: Option<String>,
    /// Whether `note` marks the row as lesser (e.g. a disabled skill), dimming the whole row.
    /// A neutral note set with [`MultiSelectItem::hint`] leaves the row at full brightness.
    dim_row: bool,
    selected: bool,
}

impl MultiSelectItem {
    /// A row identified and labeled by `key`.
    pub fn new(key: impl Into<String>) -> Self {
        let key = key.into();
        Self {
            display: sanitize(&key),
            key,
            note: None,
            dim_row: false,
            selected: false,
        }
    }

    /// Parenthesized suffix marking the row as lesser (e.g. `disabled`), which dims the whole
    /// row. Matched by the search query.
    pub fn note(mut self, note: impl Into<String>) -> Self {
        self.note = Some(sanitize(&note.into()));
        self.dim_row = true;
        self
    }

    /// Parenthesized suffix carrying neutral detail (e.g. where a row places files). Unlike
    /// [`MultiSelectItem::note`] it does not dim the row, so a list where every row has one
    /// still reads as a list of equals. Matched by the search query.
    pub fn hint(mut self, hint: impl Into<String>) -> Self {
        self.note = Some(sanitize(&hint.into()));
        self.dim_row = false;
        self
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Text the search query matches against — what the user can actually see.
    fn haystack(&self) -> String {
        match &self.note {
            Some(note) => format!("{} {}", self.display, note).to_lowercase(),
            None => self.display.to_lowercase(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Mode {
    List,
    Search,
}

/// What the caller should do after a keypress.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Step {
    Continue,
    Confirm,
    Cancel,
}

/// A full-screen checkbox list with a search filter.
///
/// ```no_run
/// # use skill_manager::tui::{MultiSelect, MultiSelectItem};
/// let keys = MultiSelect::new("Skills for profile `work`")
///     .items([
///         MultiSelectItem::new("docx").selected(true),
///         MultiSelectItem::new("pdf").note("disabled"),
///     ])
///     .interact()?;
/// # Ok::<_, skill_manager::SkmError>(())
/// ```
pub struct MultiSelect {
    state: State,
}

impl MultiSelect {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            state: State::new(sanitize(&title.into())),
        }
    }

    pub fn items(mut self, items: impl IntoIterator<Item = MultiSelectItem>) -> Self {
        self.state.items.extend(items);
        self.state.refilter();
        self
    }

    /// Take over the terminal until the user confirms or cancels.
    ///
    /// Returns the keys of the selected items in list order. Cancelling (`q`, `Esc`, `Ctrl-C`)
    /// yields [`SkmError::SelectionCancelled`]; the terminal is restored either way.
    pub fn interact(mut self) -> Result<Vec<String>, SkmError> {
        let term = Term::stderr();
        if !term.is_term() {
            return Err(SkmError::NotATty);
        }

        self.state.color = color_stderr();
        let screen = Screen::enter(&term)?;

        loop {
            let (rows, cols) = term.size();
            let lines = self.state.render(cols as usize, rows as usize);
            screen.draw(&lines)?;

            // `read_key_raw` surfaces Ctrl-C as a key instead of raising SIGINT, which would
            // skip `Screen`'s teardown and strand the user in the alternate screen.
            let key = term
                .read_key_raw()
                .map_err(|_| SkmError::SelectionCancelled)?;

            match self.state.handle(key) {
                Step::Continue => {}
                Step::Confirm => return Ok(self.state.selected_keys()),
                Step::Cancel => return Err(SkmError::SelectionCancelled),
            }
        }
    }
}

/// Widget state. Every method here is pure with respect to the terminal.
struct State {
    title: String,
    items: Vec<MultiSelectItem>,
    /// Indices into `items` matching `query`, in list order.
    filtered: Vec<usize>,
    query: String,
    /// Index into `filtered`.
    cursor: usize,
    /// First visible index into `filtered`.
    offset: usize,
    /// Item rows the last render could show; drives page-up/page-down.
    viewport: usize,
    mode: Mode,
    color: bool,
}

impl State {
    fn new(title: String) -> Self {
        Self {
            title,
            items: Vec::new(),
            filtered: Vec::new(),
            query: String::new(),
            cursor: 0,
            offset: 0,
            viewport: 10,
            mode: Mode::List,
            color: false,
        }
    }

    // ---- selection ----------------------------------------------------------------

    fn selected_keys(&self) -> Vec<String> {
        self.items
            .iter()
            .filter(|item| item.selected)
            .map(|item| item.key.clone())
            .collect()
    }

    fn selected_count(&self) -> usize {
        self.items.iter().filter(|item| item.selected).count()
    }

    fn toggle_current(&mut self) {
        if let Some(&index) = self.filtered.get(self.cursor) {
            self.items[index].selected = !self.items[index].selected;
        }
    }

    /// Select every row matching the current filter, or clear them all when they are already
    /// selected. Rows scrolled off screen still count — the filter is the scope, not the window.
    fn toggle_all_matching(&mut self) {
        if self.filtered.is_empty() {
            return;
        }
        let select = !self.filtered.iter().all(|&i| self.items[i].selected);
        for &index in &self.filtered {
            self.items[index].selected = select;
        }
    }

    // ---- filtering ----------------------------------------------------------------

    /// Recompute `filtered`, keeping the highlight on the same item when it survives.
    fn refilter(&mut self) {
        let anchor = self.filtered.get(self.cursor).copied();

        let terms: Vec<String> = self
            .query
            .split_whitespace()
            .map(str::to_lowercase)
            .collect();

        self.filtered = self
            .items
            .iter()
            .enumerate()
            .filter(|(_, item)| {
                // Short-circuit before `haystack` allocates: an empty query matches everything,
                // and that is the state on every render until the user types.
                terms.is_empty() || {
                    let haystack = item.haystack();
                    terms.iter().all(|term| haystack.contains(term))
                }
            })
            .map(|(index, _)| index)
            .collect();

        self.cursor = anchor
            .and_then(|index| self.filtered.iter().position(|&i| i == index))
            .unwrap_or(0);
        self.offset = 0;
    }

    fn set_query(&mut self, query: String) {
        self.query = query;
        self.refilter();
    }

    // ---- navigation ---------------------------------------------------------------

    fn move_by(&mut self, delta: isize) {
        if self.filtered.is_empty() {
            return;
        }
        let len = self.filtered.len() as isize;
        // Euclidean remainder: wraps in both directions, so top/bottom are neighbours.
        self.cursor = (((self.cursor as isize + delta) % len + len) % len) as usize;
    }

    fn move_page(&mut self, forward: bool) {
        if self.filtered.is_empty() {
            return;
        }
        let page = self.viewport.max(1);
        self.cursor = if forward {
            (self.cursor + page).min(self.filtered.len() - 1)
        } else {
            self.cursor.saturating_sub(page)
        };
    }

    fn move_to_edge(&mut self, last: bool) {
        if self.filtered.is_empty() {
            return;
        }
        self.cursor = if last { self.filtered.len() - 1 } else { 0 };
    }

    /// Keep `cursor` inside `[offset, offset + viewport)`.
    fn clamp_scroll(&mut self) {
        if self.filtered.is_empty() {
            self.cursor = 0;
            self.offset = 0;
            return;
        }
        self.cursor = self.cursor.min(self.filtered.len() - 1);

        if self.cursor < self.offset {
            self.offset = self.cursor;
        } else if self.cursor >= self.offset + self.viewport {
            self.offset = self.cursor + 1 - self.viewport;
        }
        self.offset = self
            .offset
            .min(self.filtered.len().saturating_sub(self.viewport));
    }

    // ---- input --------------------------------------------------------------------

    fn handle(&mut self, key: Key) -> Step {
        match self.mode {
            Mode::List => self.handle_list(key),
            Mode::Search => self.handle_search(key),
        }
    }

    fn handle_list(&mut self, key: Key) -> Step {
        match key {
            Key::ArrowDown | Key::Tab | Key::Char('j') => self.move_by(1),
            Key::ArrowUp | Key::BackTab | Key::Char('k') => self.move_by(-1),
            Key::PageDown => self.move_page(true),
            Key::PageUp => self.move_page(false),
            Key::Home | Key::Char('g') => self.move_to_edge(false),
            Key::End | Key::Char('G') => self.move_to_edge(true),
            Key::Char(' ') => self.toggle_current(),
            Key::Char('a') => self.toggle_all_matching(),
            Key::Char('/') => self.mode = Mode::Search,
            Key::Enter => return Step::Confirm,
            // Esc backs out of the filter first, so it is never a surprise quit.
            Key::Escape if !self.query.is_empty() => self.set_query(String::new()),
            Key::Escape | Key::Char('q') | Key::CtrlC => return Step::Cancel,
            _ => {}
        }
        Step::Continue
    }

    fn handle_search(&mut self, key: Key) -> Step {
        match key {
            // Enter confirms only from list mode; here it just closes the search field.
            Key::Enter | Key::Escape => self.mode = Mode::List,
            Key::ArrowDown => self.move_by(1),
            Key::ArrowUp => self.move_by(-1),
            Key::PageDown => self.move_page(true),
            Key::PageUp => self.move_page(false),
            Key::Tab => self.toggle_current(),
            Key::Backspace => {
                let mut query = self.query.clone();
                query.pop();
                self.set_query(query);
            }
            // Ctrl-U: clear the line, as in a shell.
            Key::Char('\u{15}') => self.set_query(String::new()),
            Key::CtrlC => return Step::Cancel,
            Key::Char(c) if !c.is_control() => {
                let mut query = self.query.clone();
                query.push(c);
                self.set_query(query);
            }
            _ => {}
        }
        Step::Continue
    }

    // ---- rendering ----------------------------------------------------------------

    /// Lay out the whole screen. Item rows are padded so the status and hint bar stay pinned
    /// to the bottom regardless of how many rows match.
    fn render(&mut self, width: usize, height: usize) -> Vec<String> {
        let hints = self.hint_lines(width);

        // Rows that are not item rows. The search field, status line and hint bar always earn
        // theirs; the title and the two spacers are dropped in a short terminal so that at
        // least one item row survives.
        let full_chrome = 5 + hints.len(); // title, search, spacer, spacer, status
        let roomy = height > full_chrome;
        let chrome = if roomy { full_chrome } else { 2 + hints.len() }; // search, status
        let viewport = height.saturating_sub(chrome).max(1);

        self.viewport = viewport;
        self.clamp_scroll();

        let mut lines = Vec::with_capacity(height);
        if roomy {
            lines.push(self.title_line(width));
        }
        lines.push(self.search_line(width));
        if roomy {
            lines.push(String::new());
        }
        lines.extend(self.item_lines(width, viewport));
        if roomy {
            lines.push(String::new());
        }
        lines.push(self.status_line(width));
        lines.extend(hints);

        // Last resort. Writing past the final row scrolls the alternate screen, which
        // desynchronizes the next repaint from the top-left origin `Screen::draw` assumes.
        lines.truncate(height.max(1));
        lines
    }

    fn title_line(&self, width: usize) -> String {
        let text = truncate_str(&self.title, width, "…").to_string();
        self.bold(&text)
    }

    fn search_line(&self, width: usize) -> String {
        let body = match (self.mode, self.query.is_empty()) {
            (Mode::Search, _) => format!("  Search: {}▏", self.query),
            (Mode::List, true) => "  Search:".to_string(),
            (Mode::List, false) => format!("  Search: {}", self.query),
        };
        let suffix = match (self.mode, self.query.is_empty()) {
            (Mode::Search, _) => String::new(),
            (Mode::List, true) => " (press / to search)".to_string(),
            (Mode::List, false) => " (esc to clear)".to_string(),
        };

        // Bound the body before measuring: every line `render` returns must fit the terminal,
        // or it wraps and costs more physical rows than the caller budgeted for.
        let body = truncate_str(&body, width, "…").to_string();
        let room = width.saturating_sub(measure_text_width(&body));
        let mut line = body;
        if !suffix.is_empty() && room > 0 {
            line.push_str(&self.dim(truncate_str(&suffix, room, "").as_ref()));
        }
        line
    }

    fn item_lines(&self, width: usize, viewport: usize) -> Vec<String> {
        let mut lines = Vec::with_capacity(viewport);

        if self.filtered.is_empty() {
            let message = format!("  no match for `{}`", self.query);
            lines.push(self.dim(truncate_str(&message, width, "…").as_ref()));
        } else {
            let content_width = width.saturating_sub(ITEM_GUTTER).max(1);
            for (row, &index) in self
                .filtered
                .iter()
                .skip(self.offset)
                .take(viewport)
                .enumerate()
            {
                let item = &self.items[index];
                let on_cursor = self.offset + row == self.cursor;
                lines.push(self.item_line(item, on_cursor, content_width));
            }
        }

        lines.resize(viewport, String::new());
        lines
    }

    fn item_line(&self, item: &MultiSelectItem, on_cursor: bool, content_width: usize) -> String {
        let checkbox = if item.selected { "[x]" } else { "[ ]" };
        let mut text = item.display.clone();
        if let Some(note) = &item.note {
            text.push_str(&format!("  ({note})"));
        }
        let text = truncate_str(&text, content_width, "…").to_string();

        let label = if on_cursor {
            self.bold(&text)
        } else if item.dim_row {
            self.dim(&text)
        } else {
            text
        };
        let checkbox = if item.selected {
            self.green(checkbox)
        } else {
            checkbox.to_string()
        };
        let cursor = if on_cursor {
            self.cyan("> ")
        } else {
            "  ".to_string()
        };

        format!("{cursor}{checkbox} {label}")
    }

    fn status_line(&self, width: usize) -> String {
        let total = self.items.len();
        let mut parts = vec![format!("{} selected", self.selected_count())];

        // A fraction only tells the user something once the query is narrowing the list;
        // unfiltered it would read "27 of 27" forever, which invites reading it as the number
        // of rows on screen. Scroll position is reported separately, below.
        parts.push(if self.query.is_empty() {
            format!("{total} item{}", if total == 1 { "" } else { "s" })
        } else {
            format!("{} of {total} match", self.filtered.len())
        });

        // The rows the viewport is hiding — the only cue that the list scrolls at all. One
        // segment, with each direction dropped when there is nothing that way, so the absence
        // of an arrow means "this is the whole list".
        let above = self.offset;
        let below = self
            .filtered
            .len()
            .saturating_sub(self.offset + self.viewport);
        let mut scroll = Vec::new();
        if above > 0 {
            scroll.push(format!("↑{above}"));
        }
        if below > 0 {
            scroll.push(format!("↓{below}"));
        }
        if !scroll.is_empty() {
            parts.push(scroll.join(" "));
        }

        let text = format!("  {}", parts.join(SEPARATOR));
        self.dim(truncate_str(&text, width, "…").as_ref())
    }

    fn hint_lines(&self, width: usize) -> Vec<String> {
        // `wrap_segments` keeps a segment whole even when it alone overflows, so truncate here
        // rather than trusting its width.
        wrap_segments(&self.hints(), width.saturating_sub(2))
            .iter()
            .map(|line| self.dim(truncate_str(&format!("  {line}"), width, "…").as_ref()))
            .collect()
    }

    fn hints(&self) -> Vec<&'static str> {
        match self.mode {
            Mode::List => {
                let mut hints = vec![
                    "↑/↓ or k/j move",
                    "space toggle",
                    "a toggle all",
                    "/ search",
                    "enter confirm",
                ];
                if self.query.is_empty() {
                    hints.push("q/esc quit");
                } else {
                    hints.push("esc clear filter");
                    hints.push("q quit");
                }
                hints
            }
            Mode::Search => vec![
                "type to filter",
                "↑/↓ move",
                "tab toggle",
                "ctrl-u clear",
                "enter/esc leave search",
            ],
        }
    }

    // ---- styling ------------------------------------------------------------------

    fn dim(&self, text: &str) -> String {
        if self.color {
            style(text).dim().to_string()
        } else {
            text.to_string()
        }
    }

    fn bold(&self, text: &str) -> String {
        if self.color {
            style(text).bold().to_string()
        } else {
            text.to_string()
        }
    }

    fn green(&self, text: &str) -> String {
        if self.color {
            style(text).green().to_string()
        } else {
            text.to_string()
        }
    }

    fn cyan(&self, text: &str) -> String {
        if self.color {
            style(text).cyan().to_string()
        } else {
            text.to_string()
        }
    }
}

/// Pack hint segments into as few `" · "`-joined lines as fit in `width`.
fn wrap_segments(segments: &[&str], width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();

    for segment in segments {
        let projected = if current.is_empty() {
            measure_text_width(segment)
        } else {
            measure_text_width(&current)
                + measure_text_width(SEPARATOR)
                + measure_text_width(segment)
        };
        if !current.is_empty() && projected > width {
            lines.push(std::mem::take(&mut current));
        }
        if !current.is_empty() {
            current.push_str(SEPARATOR);
        }
        current.push_str(segment);
    }

    if !current.is_empty() {
        lines.push(current);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Plain-text state (no ANSI) over the given keys, laid out for an 80x24 terminal.
    fn state(keys: &[&str]) -> State {
        let mut state = State::new("Skills".to_string());
        state.items = keys.iter().map(|key| MultiSelectItem::new(*key)).collect();
        state.refilter();
        state
    }

    fn type_query(state: &mut State, text: &str) {
        for c in text.chars() {
            state.handle(Key::Char(c));
        }
    }

    fn visible_keys(state: &State) -> Vec<&str> {
        state
            .filtered
            .iter()
            .map(|&i| state.items[i].key.as_str())
            .collect()
    }

    fn cursor_key(state: &State) -> &str {
        state.items[state.filtered[state.cursor]].key.as_str()
    }

    // ---- filtering ----------------------------------------------------------------

    #[test]
    fn search_filters_case_insensitively() {
        let mut state = state(&["docx", "PDF", "git"]);
        state.handle(Key::Char('/'));
        type_query(&mut state, "pd");
        assert_eq!(visible_keys(&state), vec!["PDF"]);
    }

    #[test]
    fn search_terms_are_anded() {
        let mut state = state(&["engineering/tdd", "engineering/review", "ops/tdd"]);
        state.handle(Key::Char('/'));
        type_query(&mut state, "eng tdd");
        assert_eq!(visible_keys(&state), vec!["engineering/tdd"]);
    }

    #[test]
    fn space_filters_instead_of_toggling_in_search_mode() {
        let mut state = state(&["docx", "git"]);
        state.handle(Key::Char('/'));
        type_query(&mut state, "d ");
        assert_eq!(state.query, "d ");
        assert_eq!(state.selected_keys(), Vec::<String>::new());
    }

    #[test]
    fn note_is_searchable() {
        let mut state = State::new("Skills".to_string());
        state.items = vec![
            MultiSelectItem::new("docx"),
            MultiSelectItem::new("pdf").note("disabled"),
        ];
        state.refilter();

        state.handle(Key::Char('/'));
        type_query(&mut state, "disabled");
        assert_eq!(visible_keys(&state), vec!["pdf"]);
    }

    #[test]
    fn selection_survives_filter_changes() {
        let mut state = state(&["docx", "git", "pdf"]);
        state.handle(Key::Char(' ')); // select docx

        state.handle(Key::Char('/'));
        type_query(&mut state, "git");
        state.handle(Key::Tab); // select git while filtered
        state.handle(Key::Escape);
        state.set_query(String::new());

        assert_eq!(state.selected_keys(), vec!["docx", "git"]);
    }

    #[test]
    fn cursor_sticks_to_its_item_while_filtering() {
        let mut state = state(&["alpha", "beta", "gamma"]);
        state.handle(Key::Char('j'));
        state.handle(Key::Char('j'));
        assert_eq!(cursor_key(&state), "gamma");

        state.handle(Key::Char('/'));
        type_query(&mut state, "a"); // all three still match
        assert_eq!(cursor_key(&state), "gamma");
    }

    #[test]
    fn cursor_resets_when_its_item_is_filtered_out() {
        let mut state = state(&["alpha", "beta"]);
        state.handle(Key::Char('j'));
        assert_eq!(cursor_key(&state), "beta");

        state.handle(Key::Char('/'));
        type_query(&mut state, "alp");
        assert_eq!(cursor_key(&state), "alpha");
    }

    #[test]
    fn backspace_and_ctrl_u_edit_the_query() {
        let mut state = state(&["docx"]);
        state.handle(Key::Char('/'));
        type_query(&mut state, "doc");

        state.handle(Key::Backspace);
        assert_eq!(state.query, "do");

        state.handle(Key::Char('\u{15}'));
        assert_eq!(state.query, "");
    }

    // ---- selection ----------------------------------------------------------------

    #[test]
    fn space_toggles_the_highlighted_item() {
        let mut state = state(&["docx", "git"]);
        state.handle(Key::Char(' '));
        assert_eq!(state.selected_keys(), vec!["docx"]);
        state.handle(Key::Char(' '));
        assert!(state.selected_keys().is_empty());
    }

    #[test]
    fn toggle_all_only_touches_visible_items() {
        let mut state = state(&["docx", "git", "pdf"]);
        state.handle(Key::Char('/'));
        type_query(&mut state, "d"); // docx, pdf
        state.handle(Key::Escape);

        state.handle(Key::Char('a'));
        assert_eq!(state.selected_keys(), vec!["docx", "pdf"]);

        state.handle(Key::Char('a'));
        assert!(state.selected_keys().is_empty());
    }

    #[test]
    fn toggle_all_selects_the_rest_when_some_are_already_selected() {
        let mut state = state(&["docx", "git"]);
        state.handle(Key::Char(' ')); // docx only
        state.handle(Key::Char('a'));
        assert_eq!(state.selected_keys(), vec!["docx", "git"]);
    }

    #[test]
    fn selected_keys_follow_list_order() {
        let mut state = state(&["a", "b", "c"]);
        state.handle(Key::Char('G'));
        state.handle(Key::Char(' ')); // c
        state.handle(Key::Home);
        state.handle(Key::Char(' ')); // a
        assert_eq!(state.selected_keys(), vec!["a", "c"]);
    }

    // ---- navigation ---------------------------------------------------------------

    #[test]
    fn vim_and_arrow_keys_both_move_and_wrap() {
        let mut state = state(&["a", "b", "c"]);

        state.handle(Key::Char('j'));
        assert_eq!(cursor_key(&state), "b");
        state.handle(Key::ArrowDown);
        assert_eq!(cursor_key(&state), "c");
        state.handle(Key::ArrowDown); // wraps
        assert_eq!(cursor_key(&state), "a");
        state.handle(Key::Char('k')); // wraps back
        assert_eq!(cursor_key(&state), "c");
        state.handle(Key::ArrowUp);
        assert_eq!(cursor_key(&state), "b");
    }

    #[test]
    fn arrows_navigate_without_leaving_search_mode() {
        let mut state = state(&["alpha", "algae"]);
        state.handle(Key::Char('/'));
        type_query(&mut state, "al");
        state.handle(Key::ArrowDown);

        assert_eq!(state.mode, Mode::Search);
        assert_eq!(cursor_key(&state), "algae");
    }

    #[test]
    fn edge_and_page_keys_clamp_to_the_list() {
        let keys: Vec<String> = (0..30).map(|i| format!("s{i:02}")).collect();
        let refs: Vec<&str> = keys.iter().map(String::as_str).collect();
        let mut state = state(&refs);
        state.viewport = 5;

        state.handle(Key::End);
        assert_eq!(cursor_key(&state), "s29");
        state.handle(Key::PageDown); // already at the end
        assert_eq!(cursor_key(&state), "s29");
        state.handle(Key::PageUp);
        assert_eq!(cursor_key(&state), "s24");
        state.handle(Key::Char('g'));
        assert_eq!(cursor_key(&state), "s00");
        state.handle(Key::PageUp);
        assert_eq!(cursor_key(&state), "s00");
    }

    #[test]
    fn navigation_on_an_empty_result_set_is_inert() {
        let mut state = state(&["docx"]);
        state.handle(Key::Char('/'));
        type_query(&mut state, "zzz");
        state.handle(Key::ArrowDown);
        state.handle(Key::Tab);

        assert!(state.filtered.is_empty());
        assert!(state.selected_keys().is_empty());
    }

    // ---- mode transitions ---------------------------------------------------------

    #[test]
    fn slash_enters_search_and_escape_leaves_it_keeping_the_filter() {
        let mut state = state(&["docx", "git"]);
        assert_eq!(state.mode, Mode::List);

        state.handle(Key::Char('/'));
        assert_eq!(state.mode, Mode::Search);
        type_query(&mut state, "doc");

        state.handle(Key::Escape);
        assert_eq!(state.mode, Mode::List);
        assert_eq!(state.query, "doc");
        assert_eq!(visible_keys(&state), vec!["docx"]);
    }

    #[test]
    fn enter_leaves_search_instead_of_confirming() {
        let mut state = state(&["docx"]);
        state.handle(Key::Char('/'));
        assert_eq!(state.handle(Key::Enter), Step::Continue);
        assert_eq!(state.mode, Mode::List);
    }

    #[test]
    fn enter_confirms_from_list_mode() {
        let mut state = state(&["docx"]);
        assert_eq!(state.handle(Key::Enter), Step::Confirm);
    }

    #[test]
    fn escape_clears_the_filter_before_it_quits() {
        let mut state = state(&["docx", "git"]);
        state.handle(Key::Char('/'));
        type_query(&mut state, "doc");
        state.handle(Key::Escape); // leaves search, filter intact

        assert_eq!(state.handle(Key::Escape), Step::Continue);
        assert_eq!(state.query, "");
        assert_eq!(visible_keys(&state), vec!["docx", "git"]);

        assert_eq!(state.handle(Key::Escape), Step::Cancel);
    }

    #[test]
    fn q_and_ctrl_c_cancel() {
        assert_eq!(state(&["docx"]).handle(Key::Char('q')), Step::Cancel);
        assert_eq!(state(&["docx"]).handle(Key::CtrlC), Step::Cancel);

        let mut searching = state(&["docx"]);
        searching.handle(Key::Char('/'));
        assert_eq!(searching.handle(Key::CtrlC), Step::Cancel);
    }

    #[test]
    fn q_stays_a_quit_while_a_filter_is_active() {
        let mut state = state(&["docx", "git"]);
        state.handle(Key::Char('/'));
        type_query(&mut state, "doc");
        state.handle(Key::Escape); // leaves search mode, filter intact
        assert_eq!(state.handle(Key::Char('q')), Step::Cancel);
    }

    // ---- rendering ----------------------------------------------------------------

    #[test]
    fn render_fills_the_screen_without_overflowing_it() {
        let keys: Vec<String> = (0..40).map(|i| format!("s{i:02}")).collect();
        let refs: Vec<&str> = keys.iter().map(String::as_str).collect();
        let mut state = state(&refs);

        for height in [6, 10, 24, 50] {
            let lines = state.render(80, height);
            assert_eq!(lines.len(), height, "height {height}");
        }
    }

    #[test]
    fn render_pins_the_status_and_hint_bar_to_the_bottom() {
        let mut state = state(&["docx", "git"]);
        let lines = state.render(80, 24);

        // Status, then the hint bar, are the last rows on screen — whatever the hints wrap to.
        let status_row = lines
            .iter()
            .rposition(|line| line.contains("selected"))
            .unwrap();
        assert!(lines[status_row].contains("0 selected"), "{lines:#?}");
        assert!(lines[status_row].contains("2 items"), "{lines:#?}");
        assert!(status_row < lines.len() - 1, "hint bar is missing");

        let hints = lines[status_row + 1..].join(" ");
        for expected in [
            "k/j move",
            "space toggle",
            "a toggle all",
            "/ search",
            "enter confirm",
            "q/esc quit",
        ] {
            assert!(
                hints.contains(expected),
                "{expected:?} missing from {hints:?}"
            );
        }
    }

    #[test]
    fn render_marks_the_cursor_and_the_checkboxes() {
        let mut state = state(&["docx", "git"]);
        state.handle(Key::Char(' ')); // select docx
        state.handle(Key::Char('j')); // cursor on git

        let lines = state.render(80, 24);
        assert!(lines.iter().any(|l| l == "  [x] docx"), "{lines:#?}");
        assert!(lines.iter().any(|l| l == "> [ ] git"), "{lines:#?}");
    }

    #[test]
    fn render_shows_the_note_and_the_search_field() {
        let mut state = State::new("Skills for profile `work`".to_string());
        state.items = vec![MultiSelectItem::new("pdf").note("disabled")];
        state.refilter();

        let lines = state.render(80, 24);
        assert_eq!(lines[0], "Skills for profile `work`");
        assert_eq!(lines[1], "  Search: (press / to search)");
        // Single item, so it also carries the cursor.
        assert!(
            lines.iter().any(|l| l == "> [ ] pdf  (disabled)"),
            "{lines:#?}"
        );

        state.handle(Key::Char('/'));
        type_query(&mut state, "pd");
        let lines = state.render(80, 24);
        assert_eq!(lines[1], "  Search: pd▏");
        assert!(lines.last().unwrap().contains("enter/esc leave search"));
    }

    #[test]
    fn render_reports_an_empty_result_set() {
        let mut state = state(&["docx"]);
        state.handle(Key::Char('/'));
        type_query(&mut state, "zzz");

        let lines = state.render(80, 24);
        assert!(
            lines.iter().any(|l| l == "  no match for `zzz`"),
            "{lines:#?}"
        );
        assert!(lines.iter().any(|l| l.contains("0 of 1 match")));
    }

    #[test]
    fn viewport_scrolls_to_keep_the_cursor_visible() {
        let keys: Vec<String> = (0..30).map(|i| format!("s{i:02}")).collect();
        let refs: Vec<&str> = keys.iter().map(String::as_str).collect();
        let mut state = state(&refs);

        let lines = state.render(80, 14);
        let viewport = state.viewport;
        assert!(viewport >= 5, "viewport {viewport}");
        assert!(lines.iter().any(|l| l.contains("s00")));

        state.handle(Key::End);
        let lines = state.render(80, 14);
        assert!(lines.iter().any(|l| l == "> [ ] s29"), "{lines:#?}");
        assert!(!lines.iter().any(|l| l.contains("s00")));
        assert!(lines
            .iter()
            .any(|l| l.contains(&format!("↑{}", 30 - viewport))));
    }

    #[test]
    fn long_labels_are_truncated_to_the_terminal_width() {
        let long = "a".repeat(200);
        let mut state = state(&[long.as_str()]);
        let lines = state.render(40, 24);

        for line in &lines {
            assert!(measure_text_width(line) <= 40, "{line}");
        }
    }

    /// The unfiltered count used to render as "N of N shown", which reads as "N rows are on
    /// screen" and never changed. Only the filter produces a fraction; only scrolling produces
    /// an arrow with the row count that way.
    #[test]
    fn status_line_distinguishes_filtering_from_scrolling() {
        let keys: Vec<String> = (0..30).map(|i| format!("s{i:02}")).collect();
        let refs: Vec<&str> = keys.iter().map(String::as_str).collect();
        let mut state = state(&refs);

        let status = |state: &mut State| {
            let lines = state.render(80, 14);
            lines
                .iter()
                .rev()
                .find(|line| line.contains("selected"))
                .unwrap()
                .clone()
        };

        // Unfiltered, parked at the top: a total, and only a down arrow.
        let line = status(&mut state);
        let viewport = state.viewport;
        assert!(line.contains("30 items"), "{line}");
        assert!(!line.contains("shown"), "{line}");
        assert!(line.contains(&format!("↓{}", 30 - viewport)), "{line}");
        assert!(!line.contains('↑'), "{line}");

        // Scrolled to the end: only an up arrow.
        state.handle(Key::End);
        let line = status(&mut state);
        assert!(line.contains(&format!("↑{}", 30 - viewport)), "{line}");
        assert!(!line.contains('↓'), "{line}");

        // Filtered but still longer than the viewport: a fraction *and* a scroll count.
        state.handle(Key::Char('/'));
        type_query(&mut state, "s1");
        let line = status(&mut state);
        assert!(line.contains("10 of 30 match"), "{line}");
        assert!(line.contains('↓'), "{line}");

        // Filtered down to something that fits: a fraction, no scroll counts at all.
        type_query(&mut state, "5");
        let line = status(&mut state);
        assert!(line.contains("1 of 30 match"), "{line}");
        assert!(!line.contains('↑') && !line.contains('↓'), "{line}");
    }

    #[test]
    fn status_line_says_item_not_items_for_one() {
        let mut state = state(&["docx"]);
        let lines = state.render(80, 24);
        assert!(lines.iter().any(|l| l.contains("1 item")), "{lines:#?}");
        assert!(!lines.iter().any(|l| l.contains("1 items")), "{lines:#?}");
    }

    /// Regression: `render` used to widen a narrow terminal to a 24-column minimum, so lines
    /// came back wider than the screen. Those wrap, consuming more physical rows than
    /// `lines.len()`, which scrolls the alternate screen and breaks the next repaint — the exact
    /// failure the height cap exists to prevent.
    #[test]
    fn narrow_terminals_never_get_lines_wider_than_the_screen() {
        let mut state = State::new("Skills for profile `work`".to_string());
        state.items = vec![
            MultiSelectItem::new("engineering/code-review").note("disabled"),
            MultiSelectItem::new("docx"),
        ];
        state.refilter();

        for width in [8, 12, 20, 23, 24, 40] {
            for height in [8, 24] {
                let lines = state.render(width, height);
                assert!(lines.len() <= height, "w{width} h{height}: {lines:#?}");
                for line in &lines {
                    assert!(
                        measure_text_width(line) <= width,
                        "w{width} h{height}: {line:?}"
                    );
                }
            }
        }
    }

    // ---- sanitizing ---------------------------------------------------------------

    /// Control characters in widget text could move the cursor, clear the screen or add rows.
    /// Callers pass allowlisted skill IDs today, but the widget must not depend on that.
    #[test]
    fn control_characters_never_reach_the_screen() {
        let mut state = State::new(sanitize("Profile \x1b[2Jwork"));
        state.items = vec![
            MultiSelectItem::new("do\x1b[31mcx"),
            MultiSelectItem::new("pdf").note("dis\nabled"),
            MultiSelectItem::new("plain"),
        ];
        state.refilter();
        state.handle(Key::Char('/'));
        type_query(&mut state, "cx");

        // color is false here, so any ESC left in the output came from the item text.
        for line in state.render(80, 24) {
            assert!(!line.contains('\x1b'), "{line:?}");
            assert!(!line.contains('\n'), "{line:?}");
            assert!(!line.chars().any(char::is_control), "{line:?}");
        }
    }

    /// The stripped character is replaced, not dropped, so a tampered row does not silently
    /// render as a different (possibly legitimate) name.
    #[test]
    fn stripped_characters_are_visible() {
        let item = MultiSelectItem::new("do\x1b[31mcx");
        assert_eq!(item.display, "do\u{fffd}[31mcx");
        assert_eq!(MultiSelectItem::new("docx").display, "docx");
    }

    /// Sanitizing is a rendering concern only. `key` is what gets written to the profile file,
    /// so mangling it here would corrupt the store.
    #[test]
    fn confirmed_keys_are_returned_verbatim() {
        let mut state = State::new("t".to_string());
        state.items = vec![MultiSelectItem::new("do\x1b[31mcx")];
        state.refilter();
        state.handle(Key::Char(' '));

        assert_eq!(state.selected_keys(), vec!["do\x1b[31mcx"]);
    }

    /// Zero-width and bidi characters are left in place: they cannot break the layout, and
    /// stripping them would mangle legitimate non-Latin labels.
    #[test]
    fn zero_width_characters_are_preserved() {
        let item = MultiSelectItem::new("caf\u{e9}\u{200b}");
        assert_eq!(item.display, "caf\u{e9}\u{200b}");
    }

    #[test]
    fn wrap_segments_packs_hints_into_the_available_width() {
        let segments = ["alpha", "beta", "gamma"];
        assert_eq!(
            wrap_segments(&segments, 80),
            vec!["alpha · beta · gamma".to_string()]
        );
        assert_eq!(
            wrap_segments(&segments, 14),
            vec!["alpha · beta".to_string(), "gamma".to_string()]
        );
        // A segment wider than the line still gets its own line rather than being dropped.
        assert_eq!(
            wrap_segments(&segments, 1),
            vec!["alpha".to_string(), "beta".to_string(), "gamma".to_string()]
        );
        assert_eq!(wrap_segments(&[], 80), vec![String::new()]);
    }
}
