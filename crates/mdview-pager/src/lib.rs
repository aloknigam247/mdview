#![forbid(unsafe_code)]

//! `mdview-pager` — the ratatui + crossterm pager for mdview terminal output.
//!
//! Consumes a stream of [`TermChunks`] on a channel and renders them in an
//! alt-screen pager with vim-flavoured keybinds, incremental search and an
//! optional follow-mode. ANSI escape sequences already baked into the chunks
//! are passed through to the terminal; sixel chunks are written directly to
//! stdout at the correct cell coordinates (bypassing ratatui's cell grid).

pub mod _stubs;
pub mod keymap;
pub mod search;

use std::io::{self};
use std::sync::mpsc::{Receiver, TryRecvError};
use std::time::Duration;

use crossterm::cursor::{MoveTo, Show};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::{execute, queue};
use ratatui::backend::CrosstermBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};
use ratatui::Terminal;

pub use _stubs::{StyleSpec, TermChunk, TermChunks, Theme};
use mdview_config::ConfigError;
use search::{Match, SearchIndex};

#[derive(Debug, thiserror::Error)]
pub enum PagerError {
    #[error("io: {0}")]
    Io(#[from] io::Error),
    #[error("keymap: {0}")]
    Keymap(#[from] keymap::KeymapError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Bottom,
    Help,
    LineDown,
    LineUp,
    PageDown,
    PageUp,
    Quit,
    SearchNext,
    SearchPrev,
    SearchStart,
    ToggleFollow,
    ToggleTheme,
    Top,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThemeSlot {
    #[default]
    Dark,
    Light,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    #[default]
    Normal,
    Search,
    Follow,
}

impl Mode {
    pub fn label(self) -> &'static str {
        match self {
            Mode::Normal => "NORMAL",
            Mode::Search => "SEARCH",
            Mode::Follow => "FOLLOW",
        }
    }
}

/// Pager state. Exposed for testing (follow-mode auto-scroll).
#[derive(Debug)]
pub struct Pager {
    chunks: TermChunks,
    index: SearchIndex,
    scroll: usize,
    viewport_rows: usize,
    mode: Mode,
    follow: bool,
    pending_g: bool,
    help: bool,
    query: String,
    matches: Vec<Match>,
    match_cursor: Option<usize>,
    filename: String,
    errors: Vec<ConfigError>,
    banner_dismissed: bool,
    theme_slot: ThemeSlot,
}

impl Pager {
    pub fn new(filename: String) -> Self {
        Pager {
            chunks: TermChunks::default(),
            index: SearchIndex::default(),
            scroll: 0,
            viewport_rows: 24,
            mode: Mode::Normal,
            follow: false,
            pending_g: false,
            help: false,
            query: String::new(),
            matches: Vec::new(),
            match_cursor: None,
            filename,
            errors: Vec::new(),
            banner_dismissed: false,
            theme_slot: ThemeSlot::Dark,
        }
    }

    pub fn theme_slot(&self) -> ThemeSlot {
        self.theme_slot
    }

    pub fn toggle_theme(&mut self) {
        self.theme_slot = match self.theme_slot {
            ThemeSlot::Dark => ThemeSlot::Light,
            ThemeSlot::Light => ThemeSlot::Dark,
        };
    }

    pub fn set_config_errors(&mut self, errors: Vec<ConfigError>) {
        self.banner_dismissed = errors.is_empty();
        self.errors = errors;
    }

    pub fn config_errors(&self) -> &[ConfigError] {
        &self.errors
    }

    pub fn banner_visible(&self) -> bool {
        !self.banner_dismissed && !self.errors.is_empty()
    }

    pub fn dismiss_banner(&mut self) {
        self.banner_dismissed = true;
    }

    /// Replace the current scrollback and rebuild the search index.
    pub fn set_chunks(&mut self, chunks: TermChunks) {
        self.index = SearchIndex::build(&chunks);
        self.chunks = chunks;
        if !self.query.is_empty() {
            self.matches = self.index.search(&self.query);
        }
        if self.follow {
            self.scroll_to_bottom();
        }
    }

    pub fn total_lines(&self) -> usize {
        self.index.line_count()
    }

    pub fn scroll(&self) -> usize {
        self.scroll
    }

    pub fn set_viewport(&mut self, rows: usize) {
        self.viewport_rows = rows.max(1);
    }

    pub fn scroll_to_bottom(&mut self) {
        let total = self.total_lines();
        self.scroll = total.saturating_sub(self.viewport_rows);
    }

    pub fn scroll_to_top(&mut self) {
        self.scroll = 0;
    }

    pub fn line_down(&mut self, n: usize) {
        let max = self.max_scroll();
        self.scroll = (self.scroll + n).min(max);
    }

    pub fn line_up(&mut self, n: usize) {
        self.scroll = self.scroll.saturating_sub(n);
    }

    fn max_scroll(&self) -> usize {
        self.total_lines().saturating_sub(self.viewport_rows)
    }

    pub fn follow_mode(&self) -> bool {
        self.follow
    }

    pub fn toggle_follow(&mut self) {
        self.follow = !self.follow;
        self.mode = if self.follow {
            Mode::Follow
        } else {
            Mode::Normal
        };
        if self.follow {
            self.scroll_to_bottom();
        }
    }

    pub fn apply(&mut self, action: Action) -> bool {
        match action {
            Action::Bottom => self.scroll_to_bottom(),
            Action::Help => self.help = !self.help,
            Action::LineDown => self.line_down(1),
            Action::LineUp => self.line_up(1),
            Action::PageDown => self.line_down(self.viewport_rows),
            Action::PageUp => self.line_up(self.viewport_rows),
            Action::Quit => return true,
            Action::SearchNext => self.next_match(1),
            Action::SearchPrev => self.next_match(-1),
            Action::SearchStart => {
                self.mode = Mode::Search;
                self.query.clear();
            }
            Action::ToggleFollow => self.toggle_follow(),
            Action::ToggleTheme => self.toggle_theme(),
            Action::Top => self.scroll_to_top(),
        }
        false
    }

    fn commit_search(&mut self) {
        self.matches = self.index.search(&self.query);
        self.match_cursor = if self.matches.is_empty() {
            None
        } else {
            Some(0)
        };
        if let Some(m) = self.matches.first() {
            self.scroll_to_match(m.line);
        }
        self.mode = if self.follow {
            Mode::Follow
        } else {
            Mode::Normal
        };
    }

    fn next_match(&mut self, dir: i32) {
        if self.matches.is_empty() {
            return;
        }
        let len = self.matches.len() as i32;
        let cur = self.match_cursor.unwrap_or(0) as i32;
        let next = (cur + dir).rem_euclid(len) as usize;
        self.match_cursor = Some(next);
        let line = self.matches[next].line;
        self.scroll_to_match(line);
    }

    fn scroll_to_match(&mut self, line: usize) {
        let max = self.max_scroll();
        self.scroll = line.saturating_sub(self.viewport_rows / 2).min(max);
    }
}

/// Start the pager event loop. Consumes `rx` until it returns
/// [`TryRecvError::Disconnected`] or the user quits.
pub fn run(rx: Receiver<TermChunks>, theme: &Theme) -> Result<(), PagerError> {
    let keymap = keymap::Keymap::load(None)?;
    let load = mdview_config::Config::load_full();
    let mut stdout = io::stdout();
    enable_raw_mode()?;
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;
    let mut pager = Pager::new(String::from("—"));
    pager.set_config_errors(load.errors);
    let result = event_loop(&mut terminal, &mut pager, &keymap, &rx, theme);

    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen, Show)?;
    result
}

fn event_loop<B: ratatui::backend::Backend + io::Write>(
    terminal: &mut Terminal<B>,
    pager: &mut Pager,
    keymap: &keymap::Keymap,
    rx: &Receiver<TermChunks>,
    theme: &Theme,
) -> Result<(), PagerError> {
    loop {
        match rx.try_recv() {
            Ok(chunks) => {
                if let Some(name) = &chunks.source_name {
                    pager.filename = name.clone();
                }
                pager.set_chunks(chunks);
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => {}
        }

        terminal.draw(|f| {
            let area = f.area();
            let banner_visible = pager.banner_visible();
            let banner_h: u16 = if banner_visible { 1 } else { 0 };
            let body_h = area.height.saturating_sub(1 + banner_h);
            pager.set_viewport(body_h as usize);
            let layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(banner_h),
                    Constraint::Min(1),
                    Constraint::Length(1),
                ])
                .split(area);
            if banner_visible {
                f.render_widget(ErrorBanner { pager }, layout[0]);
            }
            f.render_widget(AnsiView { pager, theme }, layout[1]);
            f.render_widget(StatusBar { pager }, layout[2]);
            if pager.help {
                render_help(f, area);
            }
        })?;

        draw_sixels(pager, terminal)?;

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if handle_key(pager, keymap, key) {
                    break;
                }
            }
        }
    }
    Ok(())
}

fn draw_sixels<B: ratatui::backend::Backend + io::Write>(
    pager: &Pager,
    terminal: &mut Terminal<B>,
) -> Result<(), PagerError> {
    let mut row: isize = -(pager.scroll as isize);
    let mut out = terminal.backend_mut();
    for chunk in &pager.chunks.chunks {
        match chunk {
            TermChunk::Ansi(s) => {
                let lines = s.matches('\n').count() + 1;
                row += lines as isize;
            }
            TermChunk::Sixel { payload, rows } => {
                if row >= 0 && row < pager.viewport_rows as isize {
                    queue!(out, MoveTo(0, row as u16))?;
                    out.write_all(payload.as_bytes())?;
                }
                row += *rows as isize;
            }
        }
    }
    std::io::Write::flush(&mut out)?;
    Ok(())
}

fn handle_key(pager: &mut Pager, keymap: &keymap::Keymap, key: KeyEvent) -> bool {
    if pager.banner_visible() && pager.mode != Mode::Search && key.code == KeyCode::Esc {
        pager.dismiss_banner();
        return false;
    }
    if pager.mode == Mode::Search {
        match key.code {
            KeyCode::Esc => {
                pager.query.clear();
                pager.mode = if pager.follow {
                    Mode::Follow
                } else {
                    Mode::Normal
                };
            }
            KeyCode::Enter => pager.commit_search(),
            KeyCode::Backspace => {
                pager.query.pop();
            }
            KeyCode::Char(c) => pager.query.push(c),
            _ => {}
        }
        return false;
    }

    let token = event_to_token(key);
    if let Some(tok) = token.as_deref() {
        if tok == "g" && !pager.pending_g {
            pager.pending_g = true;
            return false;
        }
        if tok == "g" && pager.pending_g {
            pager.pending_g = false;
            return pager.apply(Action::Top);
        }
        pager.pending_g = false;
        if let Some(action) = keymap.lookup(tok) {
            return pager.apply(action);
        }
    }
    false
}

fn event_to_token(key: KeyEvent) -> Option<String> {
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        if let KeyCode::Char(c) = key.code {
            return Some(format!("C-{}", c));
        }
    }
    match key.code {
        KeyCode::Char(' ') => Some("space".into()),
        KeyCode::Char(c) => Some(c.to_string()),
        KeyCode::Up => Some("Up".into()),
        KeyCode::Down => Some("Down".into()),
        KeyCode::PageUp => Some("PgUp".into()),
        KeyCode::PageDown => Some("PgDn".into()),
        KeyCode::Home => Some("Home".into()),
        KeyCode::End => Some("End".into()),
        _ => None,
    }
}

struct AnsiView<'a> {
    pager: &'a Pager,
    theme: &'a Theme,
}

impl<'a> Widget for AnsiView<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let _ = self.theme;
        let lines: Vec<Line<'_>> = self
            .pager
            .index
            .lines
            .iter()
            .skip(self.pager.scroll)
            .take(area.height as usize)
            .map(|l| Line::from(l.clone()))
            .collect();
        Paragraph::new(lines).render(area, buf);
    }
}

struct ErrorBanner<'a> {
    pager: &'a Pager,
}

impl<'a> Widget for ErrorBanner<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let errors = self.pager.config_errors();
        let first = errors.first().map(|e| e.to_string()).unwrap_or_default();
        let body = if errors.len() > 1 {
            format!(
                " ! config: {} errors \u{2014} {}  (Esc to dismiss)",
                errors.len(),
                first
            )
        } else {
            format!(" ! config: {}  (Esc to dismiss)", first)
        };
        let style = Style::default()
            .fg(Color::Black)
            .bg(Color::Yellow)
            .add_modifier(Modifier::BOLD);
        Paragraph::new(Span::styled(body, style)).render(area, buf);
    }
}

struct StatusBar<'a> {
    pager: &'a Pager,
}

impl<'a> Widget for StatusBar<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let total = self.pager.total_lines().max(1);
        let shown = (self.pager.scroll + self.pager.viewport_rows).min(total);
        let pct = (shown * 100) / total;
        let hint = match self.pager.mode {
            Mode::Search => format!("/{}", self.pager.query),
            _ => "j/k scroll · / search · F follow · ? help · q quit".into(),
        };
        let text = format!(
            " {} · {:>3}% · {} · {}",
            self.pager.filename,
            pct,
            self.pager.mode.label(),
            hint
        );
        let style = Style::default()
            .fg(Color::Black)
            .bg(Color::White)
            .add_modifier(Modifier::BOLD);
        Paragraph::new(Span::styled(text, style)).render(area, buf);
    }
}

fn render_help(f: &mut ratatui::Frame<'_>, area: Rect) {
    let body = "\
  j / k / Down / Up        scroll one line
  space / b / PgDn / PgUp  scroll one page
  gg / G                   top / bottom
  /                        search; n/N to navigate
  F                        toggle follow mode
  ?                        toggle this help
  q / Ctrl-C               quit
";
    let rect = Rect {
        x: area.x + area.width / 8,
        y: area.y + area.height / 8,
        width: area.width * 3 / 4,
        height: area.height * 3 / 4,
    };
    let block = ratatui::widgets::Block::default()
        .title(" keybinds ")
        .borders(ratatui::widgets::Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded);
    let p = Paragraph::new(body).block(block);
    f.render_widget(ratatui::widgets::Clear, rect);
    f.render_widget(p, rect);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chunks_from(s: &str) -> TermChunks {
        TermChunks {
            chunks: vec![TermChunk::Ansi(s.into())],
            source_name: Some("test.md".into()),
        }
    }

    #[test]
    fn keymap_defaults_contains_core_bindings() {
        let km = keymap::Keymap::defaults();
        assert_eq!(km.lookup("j"), Some(Action::LineDown));
        assert_eq!(km.lookup("k"), Some(Action::LineUp));
        assert_eq!(km.lookup("space"), Some(Action::PageDown));
        assert_eq!(km.lookup("/"), Some(Action::SearchStart));
        assert_eq!(km.lookup("F"), Some(Action::ToggleFollow));
        assert_eq!(km.lookup("q"), Some(Action::Quit));
        assert_eq!(km.lookup("C-c"), Some(Action::Quit));
    }

    #[test]
    fn keymap_parses_sample_toml() {
        let sample = r#"
[bindings]
"x" = "quit"
"d" = "page_down"
"#;
        let km = keymap::Keymap::from_toml_str(sample).unwrap();
        assert_eq!(km.lookup("x"), Some(Action::Quit));
        assert_eq!(km.lookup("d"), Some(Action::PageDown));
        // Defaults remain.
        assert_eq!(km.lookup("j"), Some(Action::LineDown));
    }

    #[test]
    fn keymap_rejects_unknown_action() {
        let sample = r#"
[bindings]
"x" = "bogus"
"#;
        let err = keymap::Keymap::from_toml_str(sample).unwrap_err();
        assert!(matches!(err, keymap::KeymapError::UnknownAction(_)));
    }

    #[test]
    fn search_finds_all_occurrences() {
        let sample = "hello world\nhello again\nno match here\nhello\n";
        let chunks = chunks_from(sample);
        let idx = SearchIndex::build(&chunks);
        let matches = idx.search("hello");
        assert_eq!(matches.len(), 3);
        assert_eq!(matches[0].line, 0);
        assert_eq!(matches[1].line, 1);
        assert_eq!(matches[2].line, 3);
        assert_eq!(matches[0].col, 0);
    }

    #[test]
    fn search_ignores_ansi_escapes() {
        let chunks = chunks_from("\x1b[31mhello\x1b[0m world");
        let idx = SearchIndex::build(&chunks);
        assert_eq!(idx.lines, vec!["hello world"]);
        let matches = idx.search("hello");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].col, 0);
    }

    #[test]
    fn follow_mode_auto_scrolls_on_new_content() {
        let mut pager = Pager::new("a.md".into());
        pager.set_viewport(5);
        pager.toggle_follow();
        assert!(pager.follow_mode());
        let mut text = String::new();
        for i in 0..20 {
            text.push_str(&format!("line {i}\n"));
        }
        pager.set_chunks(chunks_from(&text));
        // 21 lines total (trailing empty), viewport 5 → scroll = 16.
        assert_eq!(pager.total_lines(), 20);
        assert_eq!(pager.scroll(), 15);
    }

    #[test]
    fn follow_mode_off_preserves_scroll() {
        let mut pager = Pager::new("a.md".into());
        pager.set_viewport(5);
        let text: String = (0..20).map(|i| format!("l{i}\n")).collect();
        pager.set_chunks(chunks_from(&text));
        assert_eq!(pager.scroll(), 0);
    }

    #[test]
    fn page_down_clamps_to_max() {
        let mut pager = Pager::new("a.md".into());
        pager.set_viewport(4);
        let text: String = (0..10).map(|i| format!("l{i}\n")).collect();
        pager.set_chunks(chunks_from(&text));
        for _ in 0..20 {
            pager.apply(Action::PageDown);
        }
        assert_eq!(pager.scroll(), pager.total_lines().saturating_sub(4));
    }

    #[test]
    fn strip_ansi_handles_osc() {
        let s = "\x1b]0;title\x07text";
        assert_eq!(search::strip_ansi(s), "text");
    }

    #[test]
    fn toggle_theme_flips_slot() {
        let mut pager = Pager::new("a.md".into());
        assert_eq!(pager.theme_slot(), ThemeSlot::Dark);
        let quit = pager.apply(Action::ToggleTheme);
        assert!(!quit);
        assert_eq!(pager.theme_slot(), ThemeSlot::Light);
        pager.apply(Action::ToggleTheme);
        assert_eq!(pager.theme_slot(), ThemeSlot::Dark);
    }

    #[test]
    fn keymap_parses_toggle_theme() {
        let sample = r#"
[bindings]
"t" = "toggle_theme"
"#;
        let km = keymap::Keymap::from_toml_str(sample).unwrap();
        assert_eq!(km.lookup("t"), Some(Action::ToggleTheme));
    }

    #[test]
    fn banner_invisible_when_no_errors() {
        let mut pager = Pager::new("a.md".into());
        pager.set_config_errors(Vec::new());
        assert!(!pager.banner_visible());
    }

    #[test]
    fn banner_visible_when_errors_present_and_dismisses() {
        use mdview_config::Config;
        let errors = Config::from_toml_str_full("[keymap]\nquit = \"Ctrr+Q\"\n").errors;
        assert!(!errors.is_empty());
        let mut pager = Pager::new("a.md".into());
        pager.set_config_errors(errors);
        assert!(pager.banner_visible());
        pager.dismiss_banner();
        assert!(!pager.banner_visible());
    }

    #[test]
    fn banner_draws_into_buffer() {
        use mdview_config::Config;
        use ratatui::buffer::Buffer;
        use ratatui::layout::Rect;
        let errors = Config::from_toml_str_full("[keymap]\nquit = \"Ctrr+Q\"\n").errors;
        let mut pager = Pager::new("a.md".into());
        pager.set_config_errors(errors);
        let area = Rect::new(0, 0, 200, 1);
        let mut buf = Buffer::empty(area);
        ErrorBanner { pager: &pager }.render(area, &mut buf);
        let row: String = (0..area.width)
            .map(|x| buf[(x, 0)].symbol().chars().next().unwrap_or(' '))
            .collect();
        assert!(row.contains("config"), "row: {row:?}");
        assert!(row.contains("Esc"), "row: {row:?}");
    }
}
