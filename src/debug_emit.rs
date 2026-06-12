use serde::Serialize;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::{
    app::{App, FocusPane, ViewMode},
    markdown::{classify_document, LineKind},
};

#[derive(Clone, Debug, Serialize)]
pub struct ViewportDump {
    pub width: usize,
    pub height: usize,
    pub lines: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct StateDump {
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
    pub modified: bool,
    pub should_quit: bool,
    pub status: String,
    pub file_path: Option<String>,
    pub file_browser_cwd: String,
    pub selected_file: usize,
    pub selected_outline: usize,
    pub content: String,
}

pub fn state(app: &App) -> StateDump {
    StateDump {
        mode: app.mode.clone(),
        focus: app.focus.clone(),
        cursor_line: app.cursor_line,
        cursor_col: app.cursor_col,
        scroll: app.scroll,
        preview_scroll: app.preview_scroll,
        sidebar_visible: app.sidebar_visible,
        sidebar_collapsed: app.sidebar_collapsed,
        sidebar_files: app.sidebar_files,
        show_help: app.show_help,
        leader: app.leader,
        modified: app.modified,
        should_quit: app.should_quit,
        status: app.status.clone(),
        file_path: app.file_path.as_ref().map(|p| p.display().to_string()),
        file_browser_cwd: app.file_browser_cwd.display().to_string(),
        selected_file: app.selected_file,
        selected_outline: app.selected_outline,
        content: app.content.clone(),
    }
}

pub fn preview_lines(app: &App, width: usize, height: usize) -> ViewportDump {
    let inner_width = width.saturating_sub(2).max(1);
    let inner_height = height.saturating_sub(2).max(1);
    let mut rows = Vec::new();
    for line in app.content.split('\n').skip(app.preview_scroll) {
        let rendered = preview_plain_line(line);
        rows.extend(wrap_line(&rendered, inner_width));
        if rows.len() >= inner_height {
            break;
        }
    }
    rows.truncate(inner_height);
    ViewportDump {
        width,
        height,
        lines: rows,
    }
}

pub fn editor_lines(app: &App, width: usize, height: usize) -> ViewportDump {
    let inner_width = width.saturating_sub(2).max(1);
    let inner_height = height.saturating_sub(2).max(1);
    let lines: Vec<&str> = app.content.split('\n').collect();
    let number_w = lines.len().to_string().len().max(3);
    let mut rows = Vec::new();
    for (idx, raw) in lines.iter().enumerate().skip(app.scroll).take(inner_height) {
        let cursor_marked = if idx == app.cursor_line {
            mark_cursor(raw, app.cursor_col)
        } else {
            (*raw).to_string()
        };
        let prefix = format!("{:>width$} ", idx + 1, width = number_w);
        for (wrap_idx, wrapped) in
            wrap_line(&cursor_marked, inner_width.saturating_sub(prefix.width()))
                .into_iter()
                .enumerate()
        {
            if wrap_idx == 0 {
                rows.push(format!("{prefix}{wrapped}"));
            } else {
                rows.push(format!("{}{}", " ".repeat(prefix.width()), wrapped));
            }
            if rows.len() >= inner_height {
                break;
            }
        }
        if rows.len() >= inner_height {
            break;
        }
    }
    ViewportDump {
        width,
        height,
        lines: rows,
    }
}

pub fn sidebar_lines(app: &App, width: usize, height: usize) -> ViewportDump {
    let inner_width = width.saturating_sub(2).max(1);
    let inner_height = height.saturating_sub(2).max(1);
    let mut lines = if app.sidebar_collapsed {
        vec!["[ ]".to_string(), "".to_string(), "^X r".to_string()]
    } else if app.show_help {
        help_lines()
    } else if app.sidebar_files {
        app.file_entries()
            .into_iter()
            .enumerate()
            .map(|(i, entry)| {
                let marker = if i == app.selected_file { "> " } else { "  " };
                format!("{marker}{}", entry.name)
            })
            .collect()
    } else {
        outline_sidebar_lines(app)
    };
    lines = lines
        .into_iter()
        .flat_map(|line| wrap_line(&line, inner_width))
        .collect();
    lines.truncate(inner_height);
    ViewportDump {
        width,
        height,
        lines,
    }
}

fn outline_sidebar_lines(app: &App) -> Vec<String> {
    let stats = app.stats();
    let mut lines = vec![
        "FILE INFO".to_string(),
        "".to_string(),
        format!(
            "Path: {}",
            app.file_path
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "[No Name]".into())
        ),
        format!("Modified: {}", if app.modified { "Yes" } else { "No" }),
        format!("Lines: {}  Words: {}", stats.lines, stats.words),
        "".to_string(),
        "OUTLINE".to_string(),
        "".to_string(),
    ];
    let outline = app.outline();
    if outline.is_empty() {
        lines.push("No headings found".to_string());
    } else {
        for (idx, h) in outline.into_iter().enumerate() {
            let marker = if matches!(app.focus, FocusPane::Outline) && idx == app.selected_outline {
                "> "
            } else {
                "  "
            };
            lines.push(format!(
                "{marker}{}{}",
                "  ".repeat(h.level.saturating_sub(1) as usize),
                h.text
            ));
        }
    }
    lines
}

fn help_lines() -> Vec<String> {
    vec![
        "HELP",
        "",
        "Ctrl+X e/p  Edit / preview",
        "Ctrl+X f/o  Files / outline",
        "Ctrl+X b/r  Toggle sidebar collapse",
        "Ctrl+X u/y  Undo / redo",
        "Ctrl+X s    Save",
        "Ctrl+X q    Quit",
        "Ctrl+Q      Quit from anywhere",
        "",
        "Preview: ↑/↓ scroll",
        "Edit: arrows move, type, Enter, Backspace",
        "Files: ↑/↓ select, Enter open, Esc editor",
    ]
    .into_iter()
    .map(str::to_string)
    .collect()
}

fn preview_plain_line(raw: &str) -> String {
    match classify_document(raw).into_iter().next().map(|l| l.kind) {
        Some(LineKind::Heading(level)) => format!(
            "{} {}",
            "#".repeat(level as usize),
            raw.trim_start_matches('#').trim()
        ),
        Some(LineKind::Blockquote) => {
            format!("▌ {}", raw.trim_start().trim_start_matches('>').trim())
        }
        Some(LineKind::HorizontalRule) => "─".repeat(12),
        _ => strip_inline_markers(raw),
    }
}

fn strip_inline_markers(raw: &str) -> String {
    raw.replace("**", "").replace('`', "")
}

fn mark_cursor(raw: &str, col: usize) -> String {
    let byte = raw
        .char_indices()
        .nth(col)
        .map(|(i, _)| i)
        .unwrap_or(raw.len());
    let (before, after) = raw.split_at(byte);
    let mut out = String::new();
    out.push_str(before);
    out.push('[');
    if let Some(ch) = after.chars().next() {
        out.push(ch);
        out.push(']');
        out.push_str(&after[ch.len_utf8()..]);
    } else {
        out.push(' ');
        out.push(']');
    }
    out
}

fn wrap_line(raw: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![String::new()];
    }
    if raw.is_empty() {
        return vec![String::new()];
    }
    let mut rows = Vec::new();
    let mut current = String::new();
    let mut current_w = 0;
    for ch in raw.chars() {
        let w = ch.width().unwrap_or(0).max(1);
        if current_w > 0 && current_w + w > width {
            rows.push(current);
            current = String::new();
            current_w = 0;
        }
        current.push(ch);
        current_w += w;
    }
    rows.push(current);
    rows
}
