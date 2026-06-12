use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, Padding, Paragraph, Wrap},
    Frame,
};

use crate::{
    app::{App, FocusPane, ViewMode},
    markdown::{render_editor_line, render_preview_line},
    theme::Theme,
};

const SIDEBAR_COLLAPSED_W: u16 = 6;
const SIDEBAR_COLLAPSED_H: u16 = 3;

pub fn draw(frame: &mut Frame<'_>, app: &App, theme: &Theme) {
    let area = frame.area();
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(header_height(area.width, area.height)),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(area);
    draw_header(frame, app, theme, root[0]);
    draw_body(frame, app, theme, root[1]);
    draw_footer(frame, app, theme, root[2]);
}

fn header_height(w: u16, h: u16) -> u16 {
    if h < 14 {
        return 1;
    }
    let mut rows = if w < 90 { 3 } else { 2 };
    if w < 70 {
        rows += 1;
    }
    rows.min(3).min(h.saturating_sub(8).max(1))
}

fn draw_header(frame: &mut Frame<'_>, app: &App, theme: &Theme, area: Rect) {
    let mode = match app.mode {
        ViewMode::Preview => "PREVIEW",
        ViewMode::Edit => "EDIT",
    };
    let focus = match app.focus {
        FocusPane::Editor => "editor",
        FocusPane::Files => "files",
        FocusPane::Outline => "outline",
    };
    let title = app
        .file_path
        .as_ref()
        .and_then(|p| p.file_name())
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "[No Name]".into());
    let mode_color = if matches!(app.mode, ViewMode::Preview) {
        theme.warn
    } else {
        theme.success
    };
    let mut lines = vec![Line::from(vec![
        Span::styled(" Markdown Editor ", theme.badge(theme.accent)),
        Span::raw(" "),
        Span::styled(format!(" {mode} "), theme.badge(mode_color)),
        Span::raw(" "),
        Span::styled(format!(" {focus} "), theme.badge(theme.info)),
        Span::raw("  "),
        Span::styled("◆ ", Style::default().fg(theme.border_strong)),
        Span::styled(
            title,
            Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
        ),
        if app.modified {
            Span::styled("  ● unsaved", Style::default().fg(theme.warn))
        } else {
            Span::styled("  saved", theme.dim())
        },
    ])];
    lines.push(Line::from(vec![
        Span::styled(" ^X ", theme.badge(theme.accent)),
        Span::raw(" e edit  p preview  f files  o outline  b/r sidebar  u/y undo-redo  h help  s save  q quit"),
    ]));
    frame.render_widget(Paragraph::new(lines).style(theme.elevated()), area);
}

fn draw_body(frame: &mut Frame<'_>, app: &App, theme: &Theme, area: Rect) {
    if area.width < 2 || area.height < 1 {
        return;
    }
    let use_vertical = area.width < 90;
    let show_sidebar = app.sidebar_visible && area.height >= 8;
    if !show_sidebar {
        draw_main(frame, app, theme, area);
        return;
    }
    if use_vertical {
        let side_h = if app.sidebar_collapsed {
            SIDEBAR_COLLAPSED_H.min(area.height)
        } else {
            (area.height * 40 / 100)
                .max(5)
                .min(area.height.saturating_sub(3))
        };
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(side_h)])
            .split(area);
        draw_main(frame, app, theme, chunks[0]);
        draw_sidebar(frame, app, theme, chunks[1]);
    } else {
        let side_w = if app.sidebar_collapsed {
            SIDEBAR_COLLAPSED_W
        } else {
            (area.width * 35 / 100)
                .max(28)
                .min(area.width.saturating_sub(20))
        };
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(1), Constraint::Length(side_w)])
            .split(area);
        draw_main(frame, app, theme, chunks[0]);
        draw_sidebar(frame, app, theme, chunks[1]);
    }
}

fn draw_main(frame: &mut Frame<'_>, app: &App, theme: &Theme, area: Rect) {
    match app.mode {
        ViewMode::Preview => draw_preview(frame, app, theme, area),
        ViewMode::Edit => draw_editor(frame, app, theme, area),
    }
}

fn draw_preview(frame: &mut Frame<'_>, app: &App, theme: &Theme, area: Rect) {
    let lines: Vec<Line> = app
        .lines()
        .into_iter()
        .skip(app.preview_scroll)
        .take(area.height.saturating_sub(2) as usize)
        .map(|l| render_preview_line(l, theme))
        .collect();
    let p = Paragraph::new(lines)
        .block(
            Block::default()
                .title(" Preview ")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .padding(Padding::horizontal(1))
                .border_style(Style::default().fg(theme.border_strong)),
        )
        .style(theme.base())
        .wrap(Wrap { trim: false });
    frame.render_widget(p, area);
}

fn draw_editor(frame: &mut Frame<'_>, app: &App, theme: &Theme, area: Rect) {
    let all = app.lines();
    let visible_h = area.height.saturating_sub(2) as usize;
    let start = app.scroll.min(all.len().saturating_sub(1));
    let number_w = all.len().to_string().len().max(3);
    let mut lines = Vec::new();
    for (idx, raw) in all.into_iter().enumerate().skip(start).take(visible_h) {
        let mut spans = vec![Span::styled(
            format!("{:>width$} ", idx + 1, width = number_w),
            Style::default().fg(theme.text_muted),
        )];
        if idx == app.cursor_line {
            let (before, after) = split_at_char(raw, app.cursor_col);
            spans.push(Span::styled(
                before,
                Style::default().fg(theme.text).bg(theme.bg2),
            ));
            let cursor_char = after
                .chars()
                .next()
                .map(|c| c.to_string())
                .unwrap_or_else(|| " ".into());
            spans.push(Span::styled(
                cursor_char.clone(),
                Style::default()
                    .fg(theme.bg)
                    .bg(theme.info)
                    .add_modifier(Modifier::BOLD),
            ));
            let rest = after.get(cursor_char.len()..).unwrap_or("");
            spans.push(Span::styled(
                rest.to_string(),
                Style::default().fg(theme.text).bg(theme.bg2),
            ));
            lines.push(Line::from(spans));
        } else {
            spans.extend(render_editor_line(raw, theme).spans);
            lines.push(Line::from(spans));
        }
    }
    let p = Paragraph::new(lines)
        .block(
            Block::default()
                .title(" Editor ")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .padding(Padding::horizontal(1))
                .border_style(Style::default().fg(theme.border_strong)),
        )
        .style(theme.base());
    frame.render_widget(p, area);
}

fn split_at_char(s: &str, col: usize) -> (String, &str) {
    let byte = s.char_indices().nth(col).map(|(i, _)| i).unwrap_or(s.len());
    (s[..byte].to_string(), &s[byte..])
}

fn draw_sidebar(frame: &mut Frame<'_>, app: &App, theme: &Theme, area: Rect) {
    if app.sidebar_collapsed {
        let p = Paragraph::new(vec![Line::from("[ ]"), Line::from(""), Line::from("^X r")])
            .block(
                Block::default()
                    .title(" Side ")
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(theme.border)),
            )
            .style(theme.panel());
        frame.render_widget(p, area);
        return;
    }
    if app.show_help {
        draw_help(frame, theme, area);
    } else if app.sidebar_files {
        draw_files(frame, app, theme, area);
    } else {
        draw_outline(frame, app, theme, area);
    }
}

fn draw_outline(frame: &mut Frame<'_>, app: &App, theme: &Theme, area: Rect) {
    let stats = app.stats();
    let mut items = vec![
        Line::from(Span::styled(
            "FILE INFO",
            Style::default().fg(theme.info).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(format!(
            "Path: {}",
            app.file_path
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "[No Name]".into())
        )),
        Line::from(format!(
            "Modified: {}",
            if app.modified { "Yes" } else { "No" }
        )),
        Line::from(format!("Lines: {}  Words: {}", stats.lines, stats.words)),
        Line::from(""),
        Line::from(Span::styled(
            "OUTLINE",
            Style::default().fg(theme.info).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];
    let outline = app.outline();
    if outline.is_empty() {
        items.push(Line::from(Span::styled("No headings found", theme.dim())));
    } else {
        for (idx, h) in outline
            .into_iter()
            .take(area.height.saturating_sub(10) as usize)
            .enumerate()
        {
            let color = if h.level == 1 {
                theme.heading1
            } else if h.level == 2 {
                theme.heading2
            } else {
                theme.text
            };
            let selected = matches!(app.focus, FocusPane::Outline) && idx == app.selected_outline;
            let marker = if selected { "› " } else { "  " };
            let style = if selected {
                Style::default()
                    .fg(theme.bg)
                    .bg(theme.accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(color)
            };
            items.push(Line::from(Span::styled(
                format!(
                    "{marker}{}{}",
                    "  ".repeat(h.level.saturating_sub(1) as usize),
                    h.text
                ),
                style,
            )));
        }
    }
    let p = Paragraph::new(items)
        .block(
            Block::default()
                .title(" Outline (Ctrl+X f for files) ")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .padding(Padding::horizontal(1))
                .border_style(Style::default().fg(theme.border_strong)),
        )
        .style(theme.panel())
        .wrap(Wrap { trim: false });
    frame.render_widget(p, area);
}

fn draw_files(frame: &mut Frame<'_>, app: &App, theme: &Theme, area: Rect) {
    let entries = app.file_entries();
    let items: Vec<ListItem> = entries
        .iter()
        .enumerate()
        .take(area.height.saturating_sub(2) as usize)
        .map(|(i, e)| {
            let marker = if i == app.selected_file { "> " } else { "  " };
            let style = if i == app.selected_file {
                Style::default()
                    .fg(theme.success)
                    .add_modifier(Modifier::BOLD)
            } else if e.is_dir {
                Style::default().fg(theme.info)
            } else {
                Style::default().fg(theme.text)
            };
            ListItem::new(Line::from(Span::styled(
                format!("{marker}{}", e.name),
                style,
            )))
        })
        .collect();
    let list = List::new(items)
        .block(
            Block::default()
                .title(format!(" Files: {} ", app.file_browser_cwd.display()))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .padding(Padding::horizontal(1))
                .border_style(Style::default().fg(theme.border_strong)),
        )
        .style(theme.panel());
    frame.render_widget(list, area);
}

fn draw_help(frame: &mut Frame<'_>, theme: &Theme, area: Rect) {
    let lines = vec![
        Line::from(Span::styled(
            "HELP",
            Style::default().fg(theme.warn).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("Ctrl+X e/p  Edit / preview"),
        Line::from("Ctrl+X f/o  Files / outline"),
        Line::from("Ctrl+X b/r  Toggle sidebar collapse"),
        Line::from("Ctrl+X u/y  Undo / redo"),
        Line::from("Ctrl+X s    Save"),
        Line::from("Ctrl+X q    Quit"),
        Line::from("Ctrl+Q      Quit from anywhere"),
        Line::from(""),
        Line::from("Preview: ↑/↓ scroll"),
        Line::from("Edit: arrows move, type, Enter, Backspace"),
        Line::from("Files: ↑/↓ select, Enter open, Esc editor"),
    ];
    let p = Paragraph::new(lines)
        .block(
            Block::default()
                .title(" Help ")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .padding(Padding::horizontal(1))
                .border_style(Style::default().fg(theme.warn)),
        )
        .style(theme.panel());
    frame.render_widget(p, area);
}

fn draw_footer(frame: &mut Frame<'_>, app: &App, theme: &Theme, area: Rect) {
    let stats = app.stats();
    let mode = match app.mode {
        ViewMode::Preview => "PREVIEW",
        ViewMode::Edit => "EDIT",
    };
    let mode_color = if matches!(app.mode, ViewMode::Preview) {
        theme.warn
    } else {
        theme.success
    };
    let text = Line::from(vec![
        Span::styled(format!(" {mode} "), theme.badge(mode_color)),
        Span::raw(format!(
            "  Ln {}, Col {}  ",
            app.cursor_line + 1,
            app.cursor_col + 1
        )),
        Span::styled(
            format!("{} lines · {} words", stats.lines, stats.words),
            theme.dim(),
        ),
        Span::styled("  │  ", Style::default().fg(theme.border_strong)),
        Span::styled(&app.status, Style::default().fg(theme.text_muted)),
    ]);
    frame.render_widget(Paragraph::new(text).style(theme.elevated()), area);
}
