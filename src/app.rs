use std::{
    collections::HashMap,
    fs,
    ops::Range,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use serde::Serialize;
use unicode_width::UnicodeWidthStr;
use walkdir::WalkDir;

use crate::{
    config::AppConfig,
    markdown::{outline, OutlineItem},
    theme::Theme,
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub enum ViewMode {
    Preview,
    Edit,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub enum FocusPane {
    Editor,
    Files,
    Outline,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub enum LeaderMode {
    Edit,
    View,
    Files,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub enum CursorStyle {
    Brackets,
    Block,
    Bar,
    Underline,
    Box,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub enum ThemeSwatchStyle {
    Off,
    Circle,
    Square,
}

#[derive(Clone, Debug, Serialize)]
pub struct FileEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct DocumentStats {
    pub lines: usize,
    pub chars: usize,
    pub words: usize,
}

#[derive(Clone, Debug)]
pub struct EditSnapshot {
    pub content: String,
    pub cursor_line: usize,
    pub cursor_col: usize,
    pub scroll: usize,
    pub preview_scroll: usize,
}

pub struct App {
    pub content: String,
    pub file_path: Option<PathBuf>,
    pub modified: bool,
    pub mode: ViewMode,
    pub focus: FocusPane,
    pub cursor_line: usize,
    pub cursor_col: usize,
    pub scroll: usize,
    pub preview_scroll: usize,
    pub sidebar_visible: bool,
    pub sidebar_collapsed: bool,
    pub sidebar_files: bool,
    pub show_help: bool,
    pub leader: bool,
    pub active_leader_mode: Option<LeaderMode>,
    pub theme_picker_mode: bool,
    pub theme_picker_index: usize,
    pub theme_name: String,
    pub theme: Theme,
    pub cursor_style: CursorStyle,
    pub boxed_chrome: bool,
    pub theme_swatch_style: ThemeSwatchStyle,
    pub theme_swatch_spacing: usize,
    pub status: String,
    pub should_quit: bool,
    pub file_browser_cwd: PathBuf,
    pub selected_file: usize,
    pub dir_selections: HashMap<PathBuf, usize>,
    pub selected_outline: usize,
    pub undo_stack: Vec<EditSnapshot>,
    pub redo_stack: Vec<EditSnapshot>,
}

impl CursorStyle {
    pub fn as_config_str(&self) -> &'static str {
        match self {
            CursorStyle::Brackets => "brackets",
            CursorStyle::Block => "block",
            CursorStyle::Bar => "bar",
            CursorStyle::Underline => "underline",
            CursorStyle::Box => "box",
        }
    }

    pub fn from_config_str(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "brackets" => Some(CursorStyle::Brackets),
            "block" => Some(CursorStyle::Block),
            "bar" => Some(CursorStyle::Bar),
            "underline" => Some(CursorStyle::Underline),
            "box" => Some(CursorStyle::Box),
            _ => None,
        }
    }
}

impl ThemeSwatchStyle {
    pub fn as_config_str(&self) -> &'static str {
        match self {
            ThemeSwatchStyle::Off => "off",
            ThemeSwatchStyle::Circle => "circle",
            ThemeSwatchStyle::Square => "square",
        }
    }

    pub fn from_config_str(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "off" | "none" | "false" | "0" => Some(ThemeSwatchStyle::Off),
            "circle" | "circles" | "dot" | "dots" => Some(ThemeSwatchStyle::Circle),
            "square" | "squares" | "swatch" | "blocks" => Some(ThemeSwatchStyle::Square),
            _ => None,
        }
    }
}

impl App {
    pub fn new(path: Option<PathBuf>) -> Result<Self> {
        let cwd = std::env::current_dir()?;
        let config = if cfg!(test) && std::env::var_os("MD_EDITOR_RUST_CONFIG").is_none() {
            AppConfig::default()
        } else {
            AppConfig::load()
        };
        let theme_name = config.theme.clone().unwrap_or_else(|| "slate".into());
        let cursor_style = config.cursor_style.clone().unwrap_or(CursorStyle::Block);
        let boxed_chrome = config.boxed_chrome.unwrap_or(true);
        let theme_swatch_style = config
            .theme_swatch_style
            .clone()
            .unwrap_or(ThemeSwatchStyle::Square);
        let theme_swatch_spacing = config.theme_swatch_spacing.unwrap_or(0).min(8);
        let mut app = Self {
            content: default_content(),
            file_path: None,
            modified: false,
            mode: ViewMode::Preview,
            focus: FocusPane::Editor,
            cursor_line: 0,
            cursor_col: 0,
            scroll: 0,
            preview_scroll: 0,
            sidebar_visible: true,
            sidebar_collapsed: false,
            sidebar_files: false,
            show_help: false,
            leader: false,
            active_leader_mode: None,
            theme_picker_mode: false,
            theme_picker_index: 1,
            theme_name: theme_name.clone(),
            theme: Theme::named(&theme_name),
            cursor_style,
            boxed_chrome,
            theme_swatch_style,
            theme_swatch_spacing,
            status: "Markdown viewer: Ctrl+E edit, Ctrl+V view, Ctrl+F files".into(),
            should_quit: false,
            file_browser_cwd: cwd,
            selected_file: 0,
            dir_selections: HashMap::new(),
            selected_outline: 0,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        };
        if let Some(p) = path {
            app.open_file(&p)?;
        }
        Ok(app)
    }

    pub fn open_file(&mut self, path: &Path) -> Result<()> {
        let resolved = if path.is_absolute() {
            path.to_path_buf()
        } else {
            std::env::current_dir()?.join(path)
        };
        self.content = fs::read_to_string(&resolved)
            .with_context(|| format!("open {}", resolved.display()))?;
        self.file_path = Some(resolved.clone());
        self.file_browser_cwd = resolved.parent().unwrap_or(Path::new(".")).to_path_buf();
        self.modified = false;
        self.mode = ViewMode::Preview;
        self.focus = FocusPane::Editor;
        self.cursor_line = 0;
        self.cursor_col = 0;
        self.scroll = 0;
        self.preview_scroll = 0;
        self.selected_outline = 0;
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.status = format!("Opened {}", resolved.display());
        Ok(())
    }

    fn snapshot(&self) -> EditSnapshot {
        EditSnapshot {
            content: self.content.clone(),
            cursor_line: self.cursor_line,
            cursor_col: self.cursor_col,
            scroll: self.scroll,
            preview_scroll: self.preview_scroll,
        }
    }

    fn restore_snapshot(&mut self, snapshot: EditSnapshot) {
        self.content = snapshot.content;
        self.cursor_line = snapshot.cursor_line;
        self.cursor_col = snapshot.cursor_col;
        self.scroll = snapshot.scroll;
        self.preview_scroll = snapshot.preview_scroll;
        self.modified = true;
    }

    fn record_undo(&mut self) {
        self.undo_stack.push(self.snapshot());
        self.redo_stack.clear();
        if self.undo_stack.len() > 200 {
            self.undo_stack.remove(0);
        }
    }

    pub fn undo(&mut self) {
        if let Some(prev) = self.undo_stack.pop() {
            let current = self.snapshot();
            self.redo_stack.push(current);
            self.restore_snapshot(prev);
            self.status = "Undo".into();
        } else {
            self.status = "Nothing to undo".into();
        }
    }

    pub fn redo(&mut self) {
        if let Some(next) = self.redo_stack.pop() {
            let current = self.snapshot();
            self.undo_stack.push(current);
            self.restore_snapshot(next);
            self.status = "Redo".into();
        } else {
            self.status = "Nothing to redo".into();
        }
    }

    pub fn save(&mut self) -> Result<()> {
        let Some(path) = &self.file_path else {
            self.status = "No file path; save skipped".into();
            return Ok(());
        };
        fs::write(path, &self.content)?;
        self.modified = false;
        self.status = format!("Saved {}", path.display());
        Ok(())
    }

    pub fn lines(&self) -> Vec<&str> {
        self.content.split('\n').collect()
    }
    pub fn outline(&self) -> Vec<OutlineItem> {
        outline(&self.content)
    }
    pub fn stats(&self) -> DocumentStats {
        DocumentStats {
            lines: self.content.split('\n').count().max(1),
            chars: self.content.chars().count(),
            words: self.content.split_whitespace().count(),
        }
    }

    pub fn file_entries(&self) -> Vec<FileEntry> {
        let mut dirs = Vec::new();
        let mut files = Vec::new();
        dirs.push(FileEntry {
            name: "../".into(),
            path: self
                .file_browser_cwd
                .parent()
                .unwrap_or(&self.file_browser_cwd)
                .to_path_buf(),
            is_dir: true,
        });
        if let Ok(read) = fs::read_dir(&self.file_browser_cwd) {
            for entry in read.flatten() {
                let path = entry.path();
                let is_dir = path.is_dir();
                let name = entry.file_name().to_string_lossy().to_string();
                if is_dir {
                    dirs.push(FileEntry {
                        name: format!("{name}/"),
                        path,
                        is_dir,
                    });
                } else if matches!(
                    path.extension().and_then(|e| e.to_str()),
                    Some("md" | "markdown" | "mdx")
                ) {
                    files.push(FileEntry { name, path, is_dir });
                }
            }
        }
        dirs.sort_by(|a, b| a.name.cmp(&b.name));
        files.sort_by(|a, b| a.name.cmp(&b.name));
        dirs.extend(files);
        dirs
    }

    #[allow(dead_code)]
    pub fn visible_file_entries(&self, height: usize) -> Vec<FileEntry> {
        self.file_entries()
            .into_iter()
            .take(height.max(1))
            .collect()
    }

    pub fn move_cursor(&mut self, line_delta: isize, col_delta: isize) {
        let lines = self.lines();
        let line_count = lines.len().max(1);
        let next_line = self
            .cursor_line
            .saturating_add_signed(line_delta)
            .min(line_count - 1);
        let max_col = lines
            .get(next_line)
            .map(|s| UnicodeWidthStr::width(*s))
            .unwrap_or(0);
        let next_col = self
            .cursor_col
            .saturating_add_signed(col_delta)
            .min(max_col);
        self.cursor_line = next_line;
        self.cursor_col = next_col;
        self.keep_cursor_near_viewport();
    }

    pub fn move_page(&mut self, delta: isize) {
        self.move_cursor(delta * 10, 0);
        if delta > 0 {
            self.scroll = self.cursor_line.saturating_sub(2);
        } else {
            self.scroll = self.cursor_line;
        }
    }

    pub fn line_home(&mut self) {
        self.cursor_col = 0;
    }

    pub fn line_end(&mut self) {
        let max_col = self
            .lines()
            .get(self.cursor_line)
            .map(|s| UnicodeWidthStr::width(*s))
            .unwrap_or(0);
        self.cursor_col = max_col;
    }

    fn keep_cursor_near_viewport(&mut self) {
        if self.cursor_line < self.scroll {
            self.scroll = self.cursor_line;
        } else if self.cursor_line >= self.scroll + 12 {
            self.scroll = self.cursor_line.saturating_sub(11);
        }
    }

    pub fn insert_char(&mut self, ch: char) {
        self.record_undo();
        let mut lines: Vec<String> = self.lines().into_iter().map(str::to_string).collect();
        if lines.is_empty() {
            lines.push(String::new());
        }
        let idx = self.cursor_line.min(lines.len() - 1);
        let line = &mut lines[idx];
        let byte = byte_index_for_char_col(line, self.cursor_col);
        line.insert(byte, ch);
        self.cursor_col += 1;
        self.content = lines.join("\n");
        self.modified = true;
    }

    pub fn newline(&mut self) {
        self.record_undo();
        let mut lines: Vec<String> = self.lines().into_iter().map(str::to_string).collect();
        if lines.is_empty() {
            lines.push(String::new());
        }
        let idx = self.cursor_line.min(lines.len() - 1);
        let byte = byte_index_for_char_col(&lines[idx], self.cursor_col);
        let rest = lines[idx].split_off(byte);
        lines.insert(idx + 1, rest);
        self.cursor_line = idx + 1;
        self.cursor_col = 0;
        self.content = lines.join("\n");
        self.modified = true;
    }

    pub fn backspace(&mut self) {
        self.record_undo();
        let mut lines: Vec<String> = self.lines().into_iter().map(str::to_string).collect();
        if lines.is_empty() {
            return;
        }
        let idx = self.cursor_line.min(lines.len() - 1);
        if self.cursor_col > 0 {
            let byte = byte_index_for_char_col(&lines[idx], self.cursor_col);
            let prev = previous_char_boundary(&lines[idx], byte);
            lines[idx].replace_range(prev..byte, "");
            self.cursor_col -= 1;
        } else if idx > 0 {
            let prev_len = lines[idx - 1].chars().count();
            let current = lines.remove(idx);
            lines[idx - 1].push_str(&current);
            self.cursor_line = idx - 1;
            self.cursor_col = prev_len;
        }
        self.content = lines.join("\n");
        self.modified = true;
    }

    pub fn max_preview_scroll(&self) -> usize {
        self.lines().len().saturating_sub(1)
    }

    pub fn scroll_preview(&mut self, delta: isize) {
        self.preview_scroll = self
            .preview_scroll
            .saturating_add_signed(delta)
            .min(self.max_preview_scroll());
    }

    pub fn preview_home(&mut self) {
        self.preview_scroll = 0;
    }

    pub fn preview_end(&mut self) {
        self.preview_scroll = self.max_preview_scroll();
    }

    pub fn command(&mut self, key: char) -> Result<()> {
        self.leader = false;
        match key {
            'e' => self.enter_leader_mode(LeaderMode::Edit),
            'v' | 'p' => self.enter_leader_mode(LeaderMode::View),
            'f' => self.enter_leader_mode(LeaderMode::Files),
            'o' => self.show_outline(),
            't' => self.start_theme_picker(),
            'c' => self.cycle_cursor_style(),
            'b' | 'r' => {
                self.sidebar_collapsed = !self.sidebar_collapsed;
                self.sidebar_visible = true;
                self.status = if self.sidebar_collapsed {
                    "Sidebar: collapsed"
                } else {
                    "Sidebar: expanded"
                }
                .into();
            }
            'h' => {
                self.show_help = !self.show_help;
            }
            's' => {
                self.save()?;
            }
            'u' => self.undo(),
            'y' => self.redo(),
            'q' => {
                self.should_quit = true;
            }
            _ => {
                self.status = format!("Unknown command: {key}");
            }
        }
        Ok(())
    }

    pub fn enter_leader_mode(&mut self, mode: LeaderMode) {
        self.leader = false;
        if self.active_leader_mode == Some(mode.clone()) {
            self.exit_active_leader_mode();
            return;
        }
        self.active_leader_mode = Some(mode.clone());
        match mode {
            LeaderMode::Edit => {
                self.mode = ViewMode::Edit;
                self.focus = FocusPane::Editor;
                self.status = "Edit mode: s save, w wrap, Esc exits".into();
            }
            LeaderMode::View => {
                self.mode = ViewMode::Preview;
                self.focus = FocusPane::Editor;
                self.status =
                    "View mode: o outline, r collapse, t themes, c cursor, b borders".into();
            }
            LeaderMode::Files => {
                self.sidebar_visible = true;
                self.sidebar_files = true;
                self.focus = FocusPane::Files;
                self.status = "Files mode: arrows navigate, Enter open, Esc exits".into();
            }
        }
    }

    pub fn exit_active_leader_mode(&mut self) {
        let was = self.active_leader_mode.take();
        if matches!(was, Some(LeaderMode::Edit)) {
            self.mode = ViewMode::Preview;
        }
        if matches!(was, Some(LeaderMode::Files)) {
            self.sidebar_files = false;
            self.focus = FocusPane::Editor;
        }
        self.status = "Mode exited".into();
    }

    pub fn show_outline(&mut self) {
        self.sidebar_visible = true;
        self.sidebar_files = false;
        self.focus = FocusPane::Outline;
        self.selected_outline = self
            .selected_outline
            .min(self.outline().len().saturating_sub(1));
        self.status = "Outline focused".into();
    }

    pub fn start_theme_picker(&mut self) {
        self.sidebar_visible = true;
        self.sidebar_files = false;
        self.show_help = false;
        self.theme_picker_mode = true;
        self.theme_picker_index = theme_options()
            .iter()
            .position(|name| *name == self.theme_name)
            .unwrap_or(0);
        self.status = "Theme picker: ↑/↓ select, Enter apply, Esc cancel".into();
    }

    pub fn cancel_theme_picker(&mut self) {
        self.theme_picker_mode = false;
        self.status = "Theme picker cancelled".into();
    }

    pub fn move_theme_selection(&mut self, delta: isize) {
        let options = theme_options();
        if options.is_empty() {
            return;
        }
        let len = options.len() as isize;
        let next = (self.theme_picker_index as isize + delta).rem_euclid(len);
        self.theme_picker_index = next as usize;
    }

    pub fn apply_selected_theme_preview(&mut self) {
        let options = theme_options();
        let Some(name) = options.get(self.theme_picker_index).cloned() else {
            return;
        };
        self.theme_name = name.clone();
        self.theme = Theme::named(&name);
        self.theme_picker_mode = true;
        self.persist_preferences();
        self.status = format!("Theme preview -> {name}");
    }

    pub fn toggle_boxed_chrome(&mut self) {
        self.boxed_chrome = !self.boxed_chrome;
        self.persist_preferences();
        self.status = if self.boxed_chrome {
            "Chrome: boxed panels"
        } else {
            "Chrome: smooth document"
        }
        .into();
    }

    pub fn cycle_cursor_style(&mut self) {
        self.cursor_style = match self.cursor_style {
            CursorStyle::Brackets => CursorStyle::Block,
            CursorStyle::Block => CursorStyle::Bar,
            CursorStyle::Bar => CursorStyle::Underline,
            CursorStyle::Underline => CursorStyle::Box,
            CursorStyle::Box => CursorStyle::Brackets,
        };
        self.persist_preferences();
        self.status = format!("Cursor: {:?}", self.cursor_style);
    }

    fn persist_preferences(&mut self) {
        if cfg!(test) && std::env::var_os("MD_EDITOR_RUST_CONFIG").is_none() {
            return;
        }
        let cfg = AppConfig {
            theme: Some(self.theme_name.clone()),
            cursor_style: Some(self.cursor_style.clone()),
            boxed_chrome: Some(self.boxed_chrome),
            theme_swatch_style: Some(self.theme_swatch_style.clone()),
            theme_swatch_spacing: Some(self.theme_swatch_spacing),
        };
        if let Err(err) = cfg.save() {
            self.status = format!("Config save failed: {err}");
        }
    }

    pub fn jump_to_selected_outline(&mut self) {
        let outline = self.outline();
        let Some(item) = outline.get(self.selected_outline) else {
            self.status = "No heading selected".into();
            return;
        };
        self.cursor_line = item.line;
        self.cursor_col = 0;
        self.scroll = item.line.saturating_sub(2);
        self.preview_scroll = item.line.saturating_sub(2);
        self.focus = FocusPane::Editor;
        self.status = format!("Jumped to {}", item.text);
    }

    pub fn open_selected_file(&mut self) -> Result<()> {
        let entries = self.file_entries();
        let Some(entry) = entries.get(self.selected_file).cloned() else {
            return Ok(());
        };
        if entry.is_dir {
            self.change_file_browser_dir(entry.path);
        } else {
            self.open_file(&entry.path)?;
        }
        Ok(())
    }

    pub fn file_browser_parent(&mut self) {
        if let Some(parent) = self.file_browser_cwd.parent() {
            self.change_file_browser_dir(parent.to_path_buf());
        }
    }

    fn change_file_browser_dir(&mut self, path: PathBuf) {
        self.dir_selections
            .insert(self.file_browser_cwd.clone(), self.selected_file);
        self.file_browser_cwd = path;
        let max = self.file_entries().len().saturating_sub(1);
        self.selected_file = self
            .dir_selections
            .get(&self.file_browser_cwd)
            .copied()
            .unwrap_or(0)
            .min(max);
        self.status = format!("Files: {}", self.file_browser_cwd.display());
    }
}

fn byte_index_for_char_col(s: &str, col: usize) -> usize {
    s.char_indices().nth(col).map(|(i, _)| i).unwrap_or(s.len())
}
fn previous_char_boundary(s: &str, byte: usize) -> usize {
    s[..byte].char_indices().last().map(|(i, _)| i).unwrap_or(0)
}

pub fn theme_options() -> Vec<String> {
    let mut names = std::collections::BTreeSet::new();
    names.insert("ember".to_string());
    names.insert("slate".to_string());
    for dir in theme_dirs() {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("toml") {
                    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                        if stem != "config" {
                            names.insert(stem.to_string());
                        }
                    }
                }
            }
        }
    }
    names.into_iter().collect()
}

pub fn theme_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Ok(cwd) = std::env::current_dir() {
        dirs.push(cwd.join("themes"));
        dirs.push(cwd.join("../md-editor/themes"));
    }
    if let Some(home) = std::env::var_os("HOME") {
        dirs.push(PathBuf::from(home).join(".config/md-editor/themes"));
    }
    dirs
}

fn default_content() -> String {
    "# Welcome to md-editor-rust\n\nA Rust/Ratatui parity port of md-editor.\n\n## Getting Started\n\n- Starts in preview mode\n- Ctrl+X then e to edit\n- Ctrl+X then p for preview\n- Ctrl+X then f for files\n- Ctrl+X then h for help".into()
}

pub fn selected_window(total: usize, selected: usize, height: usize) -> Range<usize> {
    if total == 0 || height == 0 {
        return 0..0;
    }
    let visible = height.min(total);
    let selected = selected.min(total - 1);
    let half = visible / 2;
    let mut start = selected.saturating_sub(half);
    start = start.min(total.saturating_sub(visible));
    start..start + visible
}

#[allow(dead_code)]
pub fn discover_markdown_files(root: &Path, limit: usize) -> Vec<PathBuf> {
    WalkDir::new(root)
        .max_depth(3)
        .into_iter()
        .flatten()
        .filter(|e| e.file_type().is_file())
        .map(|e| e.into_path())
        .filter(|p| {
            matches!(
                p.extension().and_then(|e| e.to_str()),
                Some("md" | "markdown" | "mdx")
            )
        })
        .take(limit)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn edits_text_like_basic_md_editor_operations() {
        let mut app = App::new(None).unwrap();
        app.content = "abc".into();
        app.cursor_col = 1;
        app.insert_char('X');
        assert_eq!(app.content, "aXbc");
        app.backspace();
        assert_eq!(app.content, "abc");
        app.newline();
        assert_eq!(app.content, "a\nbc");
    }
    #[test]
    fn stats_match_expected() {
        let mut app = App::new(None).unwrap();
        app.content = "one two\nthree".into();
        let stats = app.stats();
        assert_eq!(stats.lines, 2);
        assert_eq!(stats.words, 3);
    }

    #[test]
    fn undo_redo_round_trip() {
        let mut app = App::new(None).unwrap();
        app.content = "abc".into();
        app.cursor_col = 3;
        app.insert_char('d');
        assert_eq!(app.content, "abcd");
        app.undo();
        assert_eq!(app.content, "abc");
        app.redo();
        assert_eq!(app.content, "abcd");
    }

    #[test]
    fn selected_window_keeps_selection_visible_and_centered_when_possible() {
        assert_eq!(selected_window(0, 0, 10), 0..0);
        assert_eq!(selected_window(20, 0, 5), 0..5);
        assert_eq!(selected_window(20, 10, 5), 8..13);
        assert_eq!(selected_window(20, 19, 5), 15..20);
        assert_eq!(selected_window(3, 2, 10), 0..3);
    }

    #[test]
    fn preview_scroll_is_bounded() {
        let mut app = App::new(None).unwrap();
        app.content = "a\nb\nc".into();
        app.scroll_preview(99);
        assert_eq!(app.preview_scroll, 2);
        app.scroll_preview(-99);
        assert_eq!(app.preview_scroll, 0);
        app.preview_end();
        assert_eq!(app.preview_scroll, 2);
        app.preview_home();
        assert_eq!(app.preview_scroll, 0);
    }

    #[test]
    fn file_browser_restores_selection_per_directory() {
        let root = std::env::temp_dir().join(format!(
            "md-editor-rust-file-selection-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("a_dir")).unwrap();
        fs::create_dir_all(root.join("b_dir")).unwrap();
        let mut app = App::new(None).unwrap();
        app.file_browser_cwd = root.clone();
        let entries = app.file_entries();
        let b_index = entries.iter().position(|e| e.name == "b_dir/").unwrap();
        app.selected_file = b_index;
        app.open_selected_file().unwrap();
        assert_eq!(app.file_browser_cwd, root.join("b_dir"));
        app.file_browser_parent();
        assert_eq!(app.file_browser_cwd, root);
        assert_eq!(app.selected_file, b_index);
        let _ = fs::remove_dir_all(app.file_browser_cwd);
    }
}
