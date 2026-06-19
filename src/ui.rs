use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, Padding, Paragraph, Wrap},
    Frame,
};

use crate::{
    app::{
        selected_window, theme_options, App, CursorStyle, FocusPane, LeaderMode, ThemeSwatchStyle,
        ViewMode,
    },
    image_view::ImageManager,
    markdown::{render_editor_line, render_preview_document_full},
    theme::Theme,
};

const SIDEBAR_COLLAPSED_W: u16 = 6;
const SIDEBAR_COLLAPSED_H: u16 = 3;

pub fn draw(frame: &mut Frame<'_>, app: &App, theme: &Theme, images: &mut ImageManager) {
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
    draw_body(frame, app, theme, root[1], images);
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
    let mode = mode_label(app);
    let mode_color = mode_color(app, theme);
    let title = app
        .file_path
        .as_ref()
        .and_then(|p| p.file_name())
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "[No Name]".into());
    let mut lines = vec![Line::from(vec![
        Span::styled(format!(" {mode} "), theme.badge(mode_color)),
        Span::raw("  "),
        Span::styled("poneglyph ", Style::default().fg(theme.text_muted)),
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
        Span::styled("  │  ", Style::default().fg(theme.border_strong)),
        Span::styled(format!("theme:{}", app.theme_name), theme.dim()),
        Span::styled("  ", Style::default()),
        cursor_badge(app, theme),
        Span::styled("  ", Style::default()),
        Span::styled(
            if app.boxed_chrome { "boxed" } else { "smooth" },
            theme.dim(),
        ),
    ])];
    lines.push(header_action_line(app, theme));
    if area.height > 2 {
        lines.push(Line::from(Span::styled(&app.status, theme.dim())));
    }
    frame.render_widget(Paragraph::new(lines).style(theme.elevated()), area);
}

fn mode_label(app: &App) -> &'static str {
    if let Some(mode) = &app.active_leader_mode {
        return match mode {
            LeaderMode::Edit => "EDIT",
            LeaderMode::View => "VIEW",
            LeaderMode::Files => "FILES",
        };
    }
    if app.leader {
        return "LEADER";
    }
    if app.theme_picker_mode {
        return "THEME";
    }
    if matches!(app.focus, FocusPane::Files) {
        return "FILES";
    }
    match app.mode {
        ViewMode::Preview => "PREVIEW",
        ViewMode::Edit => "EDIT",
    }
}

fn mode_color(app: &App, theme: &Theme) -> ratatui::style::Color {
    match app.active_leader_mode {
        Some(LeaderMode::Edit) => theme.success,
        Some(LeaderMode::View) => theme.warn,
        Some(LeaderMode::Files) => theme.info,
        None if app.leader => theme.accent,
        None if app.theme_picker_mode => theme.image,
        None if matches!(app.focus, FocusPane::Files) => theme.info,
        None if matches!(app.mode, ViewMode::Edit) => theme.success,
        None => theme.warn,
    }
}

fn cursor_badge<'a>(app: &App, theme: &Theme) -> Span<'a> {
    let label = match app.cursor_style {
        CursorStyle::Brackets => "cursor:brackets",
        CursorStyle::Block => "cursor:block",
        CursorStyle::Bar => "cursor:bar",
        CursorStyle::Underline => "cursor:underline",
        CursorStyle::Box => "cursor:box",
    };
    Span::styled(label.to_string(), theme.dim())
}

fn header_action_line<'a>(app: &App, theme: &Theme) -> Line<'a> {
    let spans = if app.leader {
        vec![
            Span::styled(" ^E ", theme.badge(theme.success)),
            Span::raw(" Edit  "),
            Span::styled(" ^V ", theme.badge(theme.warn)),
            Span::raw(" View  "),
            Span::styled(" ^F ", theme.badge(theme.info)),
            Span::raw(" Files  "),
            Span::styled(" Esc ", theme.badge(theme.text_muted)),
            Span::raw(" Cancel"),
        ]
    } else if let Some(mode) = &app.active_leader_mode {
        match mode {
            LeaderMode::Edit => vec![
                Span::styled(" s ", theme.badge(theme.success)),
                Span::raw(" Save  "),
                Span::styled(" w ", theme.badge(theme.success)),
                Span::raw(" Wrap  "),
                Span::styled(" Esc/q ", theme.badge(theme.text_muted)),
                Span::raw(" Exit"),
            ],
            LeaderMode::View => vec![
                Span::styled(" o ", theme.badge(theme.warn)),
                Span::raw(" Outline  "),
                Span::styled(" r ", theme.badge(theme.warn)),
                Span::raw(" Collapse  "),
                Span::styled(" t ", theme.badge(theme.image)),
                Span::raw(" Themes  "),
                Span::styled(" b ", theme.badge(theme.accent)),
                Span::raw(" Borders  "),
                Span::styled(" c ", theme.badge(theme.info)),
                Span::raw(" Cursor  "),
                Span::styled(" Esc/q ", theme.badge(theme.text_muted)),
                Span::raw(" Exit"),
            ],
            LeaderMode::Files => vec![
                Span::styled(" ↑↓ ", theme.badge(theme.info)),
                Span::raw(" Nav  "),
                Span::styled(" Enter ", theme.badge(theme.info)),
                Span::raw(" Open  "),
                Span::styled(" ← ", theme.badge(theme.info)),
                Span::raw(" Parent  "),
                Span::styled(" Esc/q ", theme.badge(theme.text_muted)),
                Span::raw(" Exit"),
            ],
        }
    } else if matches!(app.focus, FocusPane::Files) {
        vec![
            Span::styled(" ↑↓ ", theme.badge(theme.info)),
            Span::raw(" Nav  "),
            Span::styled(" Enter ", theme.badge(theme.info)),
            Span::raw(" Open  "),
            Span::styled(" Esc ", theme.badge(theme.text_muted)),
            Span::raw(" Editor  "),
            Span::styled(" ^E/^V/^F ", theme.badge(theme.accent)),
            Span::raw(" Modes"),
        ]
    } else {
        vec![
            Span::styled(" ^E ", theme.badge(theme.success)),
            Span::raw(" Edit  "),
            Span::styled(" ^V ", theme.badge(theme.warn)),
            Span::raw(" View  "),
            Span::styled(" ^F ", theme.badge(theme.info)),
            Span::raw(" Files"),
        ]
    };
    Line::from(spans)
}

fn draw_body(
    frame: &mut Frame<'_>,
    app: &App,
    theme: &Theme,
    area: Rect,
    images: &mut ImageManager,
) {
    if area.width < 2 || area.height < 1 {
        return;
    }
    let use_vertical = area.width < 90;
    let show_sidebar = app.sidebar_visible && area.height >= 8;
    if !show_sidebar {
        draw_main(frame, app, theme, area, images);
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
        if app.boxed_chrome {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(1), Constraint::Length(side_h)])
                .split(area);
            draw_main(frame, app, theme, chunks[0], images);
            draw_sidebar(frame, app, theme, chunks[1]);
        } else {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(1),
                    Constraint::Length(1),
                    Constraint::Length(side_h),
                ])
                .split(area);
            draw_main(frame, app, theme, chunks[0], images);
            draw_horizontal_separator(frame, theme, chunks[1]);
            draw_sidebar(frame, app, theme, chunks[2]);
        }
    } else {
        let side_w = if app.sidebar_collapsed {
            SIDEBAR_COLLAPSED_W
        } else {
            (area.width * 35 / 100)
                .max(28)
                .min(area.width.saturating_sub(20))
        };
        if app.boxed_chrome {
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Min(1), Constraint::Length(side_w)])
                .split(area);
            draw_main(frame, app, theme, chunks[0], images);
            draw_sidebar(frame, app, theme, chunks[1]);
        } else {
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Min(1),
                    Constraint::Length(1),
                    Constraint::Length(side_w),
                ])
                .split(area);
            draw_main(frame, app, theme, chunks[0], images);
            draw_vertical_separator(frame, theme, chunks[1]);
            draw_sidebar(frame, app, theme, chunks[2]);
        }
    }
}

fn draw_vertical_separator(frame: &mut Frame<'_>, theme: &Theme, area: Rect) {
    let rows = vec![
        Line::from(Span::styled("│", Style::default().fg(theme.border)));
        area.height as usize
    ];
    frame.render_widget(Paragraph::new(rows).style(theme.base()), area);
}

fn draw_horizontal_separator(frame: &mut Frame<'_>, theme: &Theme, area: Rect) {
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "─".repeat(area.width as usize),
            Style::default().fg(theme.border),
        )))
        .style(theme.base()),
        area,
    );
}

fn draw_main(
    frame: &mut Frame<'_>,
    app: &App,
    theme: &Theme,
    area: Rect,
    images: &mut ImageManager,
) {
    match app.mode {
        ViewMode::Preview => draw_preview(frame, app, theme, area, images),
        ViewMode::Edit => draw_editor(frame, app, theme, area),
    }
}

fn draw_preview(
    frame: &mut Frame<'_>,
    app: &App,
    theme: &Theme,
    area: Rect,
    images: &mut ImageManager,
) {
    let render_area = smooth_content_area(app, area);
    let take = render_area
        .height
        .saturating_sub(if app.boxed_chrome { 2 } else { 0 }) as usize;
    let width = render_area
        .width
        .saturating_sub(if app.boxed_chrome { 4 } else { 1 }) as usize;
    // Size each image block to its aspect ratio capped to the viewport so it never needs cropping
    let supported = images.supported();
    let max_rows = (take as u16).max(1);
    let avail = width as u16;
    let (all_lines, placements) = {
        let mut image_rows = |target: &str, _w: usize| -> usize {
            if !supported {
                return 2;
            }
            match images.fit_size(target, avail, max_rows) {
                Some(size) => size.height as usize,
                None => 2,
            }
        };
        render_preview_document_full(&app.content, width, theme, &mut image_rows)
    };
    let lines: Vec<Line> = all_lines
        .iter()
        .skip(app.preview_scroll)
        .take(take)
        .cloned()
        .collect();
    // Base the percent on the rendered line count which includes expanded image rows
    let max_scroll = all_lines.len().saturating_sub(1).max(1);
    let percent = ((app.preview_scroll * 100) / max_scroll).min(100);
    let mut p = Paragraph::new(lines)
        .style(theme.base())
        .wrap(Wrap { trim: false });
    if app.boxed_chrome {
        p = p.block(
            Block::default()
                .title(format!(" Preview · {}% ", percent))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .padding(Padding::horizontal(1))
                .border_style(Style::default().fg(theme.border_strong)),
        );
    }
    frame.render_widget(p, render_area);

    overlay_images(
        frame,
        app,
        theme,
        images,
        render_area,
        take,
        width,
        &placements,
    );
}

#[allow(clippy::too_many_arguments)]
fn overlay_images(
    frame: &mut Frame<'_>,
    app: &App,
    theme: &Theme,
    images: &mut ImageManager,
    render_area: Rect,
    take: usize,
    width: usize,
    placements: &[crate::markdown::ImagePlacement],
) {
    if width == 0 || take == 0 {
        return;
    }
    let content_x = render_area.x + if app.boxed_chrome { 2 } else { 0 };
    let content_y = render_area.y + if app.boxed_chrome { 1 } else { 0 };
    let content_w = width as u16;
    let content_h = take as u16;
    let supported = images.supported();
    let view_end = app.preview_scroll + content_h as usize;
    for placement in placements {
        // Skip blocks that don't overlap the visible window at all
        let placement_end = placement.start + placement.rows as usize;
        if placement_end <= app.preview_scroll || placement.start >= view_end {
            continue;
        }
        // Clamp the top row to the viewport for blocks that scrolled in from above
        let row = placement.start.saturating_sub(app.preview_scroll) as u16;

        if !supported {
            render_image_note(
                frame,
                theme,
                "image preview not supported in this terminal",
                content_x,
                content_y,
                content_w,
                row,
                content_h,
            );
            continue;
        }

        // Terminal graphics can't clip the source so only draw when the whole block fits
        let fully_visible = placement.start >= app.preview_scroll;
        match images.fit_size(&placement.target, content_w, content_h) {
            Some(size) => {
                let height = size.height.min(placement.rows);
                if !fully_visible || row + height > content_h {
                    // Partially scrolled blocks show a hint instead of a blank gap
                    render_image_note(
                        frame,
                        theme,
                        "image clipped — scroll to view",
                        content_x,
                        content_y,
                        content_w,
                        row,
                        content_h,
                    );
                    continue;
                }
                let rect = Rect {
                    x: content_x,
                    y: content_y + row,
                    width: size.width.min(content_w),
                    height,
                };
                images.render(frame, &placement.target, rect);
            }
            None => render_image_note(
                frame,
                theme,
                "image unavailable",
                content_x,
                content_y,
                content_w,
                row,
                content_h,
            ),
        }
    }
}

/// One-line hint drawn just below an image's fallback icon row
#[allow(clippy::too_many_arguments)]
fn render_image_note(
    frame: &mut Frame<'_>,
    theme: &Theme,
    text: &str,
    content_x: u16,
    content_y: u16,
    content_w: u16,
    row: u16,
    content_h: u16,
) {
    let note_row = row + 1;
    if note_row >= content_h {
        return;
    }
    let rect = Rect {
        x: content_x,
        y: content_y + note_row,
        width: content_w,
        height: 1,
    };
    let note = Paragraph::new(Line::from(Span::styled(
        format!("  {text}"),
        Style::default()
            .fg(theme.warn)
            .add_modifier(Modifier::ITALIC),
    )))
    .style(theme.base());
    frame.render_widget(note, rect);
}

fn smooth_content_area(app: &App, area: Rect) -> Rect {
    if app.boxed_chrome || area.width < 6 {
        area
    } else {
        Rect {
            x: area.x.saturating_add(2),
            y: area.y,
            width: area.width.saturating_sub(3),
            height: area.height,
        }
    }
}

fn draw_editor(frame: &mut Frame<'_>, app: &App, theme: &Theme, area: Rect) {
    let area = smooth_content_area(app, area);
    let all = app.lines();
    let visible_h = area
        .height
        .saturating_sub(if app.boxed_chrome { 2 } else { 0 }) as usize;
    let start = app.scroll.min(all.len().saturating_sub(1));
    let number_w = all.len().to_string().len().max(3);
    let mut lines = Vec::new();
    for (idx, raw) in all.iter().copied().enumerate().skip(start).take(visible_h) {
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
                cursor_glyph(app, &cursor_char),
                cursor_style(app, theme),
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
    let visible_end = (start + visible_h).min(all.len());
    let mut p = Paragraph::new(lines).style(theme.base());
    if app.boxed_chrome {
        p = p.block(
            Block::default()
                .title(format!(
                    " Editor · lines {}-{} / {} ",
                    start + 1,
                    visible_end,
                    all.len()
                ))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .padding(Padding::horizontal(1))
                .border_style(Style::default().fg(theme.border_strong)),
        );
    }
    frame.render_widget(p, area);
}

fn cursor_glyph(app: &App, ch: &str) -> String {
    match app.cursor_style {
        CursorStyle::Bar => format!("▏{ch}"),
        CursorStyle::Underline => format!("{ch}̲"),
        CursorStyle::Box => format!("[{ch}]"),
        CursorStyle::Brackets => format!("[{ch}]"),
        CursorStyle::Block => ch.to_string(),
    }
}

fn cursor_style(app: &App, theme: &Theme) -> Style {
    match app.cursor_style {
        CursorStyle::Bar | CursorStyle::Underline | CursorStyle::Brackets | CursorStyle::Box => {
            Style::default().fg(theme.info).add_modifier(Modifier::BOLD)
        }
        CursorStyle::Block => Style::default()
            .fg(theme.bg)
            .bg(theme.info)
            .add_modifier(Modifier::BOLD),
    }
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
    if app.theme_picker_mode {
        draw_theme_picker(frame, app, theme, area);
    } else if app.show_help {
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
        let heading_rows = area.height.saturating_sub(10) as usize;
        let window = selected_window(outline.len(), app.selected_outline, heading_rows);
        if window.start > 0 {
            items.push(Line::from(Span::styled(
                format!("  ↑ {} more", window.start),
                theme.dim(),
            )));
        }
        for (idx, h) in outline
            .iter()
            .enumerate()
            .skip(window.start)
            .take(window.len())
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
        if window.end < outline.len() {
            items.push(Line::from(Span::styled(
                format!("  ↓ {} more", outline.len() - window.end),
                theme.dim(),
            )));
        }
    }
    if !app.boxed_chrome {
        items.insert(
            0,
            Line::from(Span::styled(
                "Outline",
                Style::default().fg(theme.info).add_modifier(Modifier::BOLD),
            )),
        );
        items.insert(
            1,
            Line::from(Span::styled("─".repeat(area.width as usize), theme.dim())),
        );
    }
    let mut p = Paragraph::new(items)
        .style(theme.panel())
        .wrap(Wrap { trim: false });
    if app.boxed_chrome {
        p = p.block(
            Block::default()
                .title(" Outline (Ctrl+X f for files) ")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .padding(Padding::horizontal(1))
                .border_style(Style::default().fg(theme.border_strong)),
        );
    }
    frame.render_widget(p, area);
}

fn theme_swatch_spans<'a>(app: &App, preview_theme: &Theme) -> Vec<Span<'a>> {
    let glyph = match app.theme_swatch_style {
        ThemeSwatchStyle::Off => return Vec::new(),
        ThemeSwatchStyle::Circle => "●",
        ThemeSwatchStyle::Square => "■",
    };
    let gap = " ".repeat(app.theme_swatch_spacing.min(8));
    let colors = [
        preview_theme.heading1,
        preview_theme.info,
        preview_theme.warn,
    ];
    let mut spans = Vec::new();
    for (idx, color) in colors.into_iter().enumerate() {
        if idx > 0 && !gap.is_empty() {
            spans.push(Span::raw(gap.clone()));
        }
        spans.push(Span::styled(glyph.to_string(), Style::default().fg(color)));
    }
    spans.push(Span::raw(" "));
    spans
}

fn draw_theme_picker(frame: &mut Frame<'_>, app: &App, theme: &Theme, area: Rect) {
    let options = theme_options();
    let reserved_rows = if app.boxed_chrome { 8 } else { 9 };
    let visible_rows = (area.height as usize).saturating_sub(reserved_rows).max(1);
    let window = selected_window(options.len(), app.theme_picker_index, visible_rows);
    let mut lines = vec![
        Line::from(Span::styled(
            "THEME SELECTOR",
            Style::default().fg(theme.info).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            format!(
                "{} themes · {} selected",
                options.len(),
                options
                    .get(app.theme_picker_index)
                    .map(String::as_str)
                    .unwrap_or("none")
            ),
            theme.dim(),
        )),
        Line::from(""),
    ];
    if window.start > 0 {
        lines.push(Line::from(Span::styled(
            format!("  ↑ {} more", window.start),
            theme.dim(),
        )));
    }
    for (idx, name) in options
        .iter()
        .enumerate()
        .skip(window.start)
        .take(window.len())
    {
        let selected = idx == app.theme_picker_index;
        let current = name == &app.theme_name;
        let marker = if selected { "›" } else { " " };
        let current_marker = if current { " *" } else { "" };
        let preview_theme = Theme::named(name);
        let style = if selected {
            Style::default()
                .fg(theme.bg)
                .bg(theme.image)
                .add_modifier(Modifier::BOLD)
        } else if current {
            Style::default().fg(theme.success)
        } else {
            Style::default().fg(theme.text)
        };
        let mut row = vec![Span::styled(format!("{marker} "), style)];
        row.extend(theme_swatch_spans(app, &preview_theme));
        row.push(Span::styled(format!("{name}{current_marker}"), style));
        lines.push(Line::from(row));
    }
    if window.end < options.len() {
        lines.push(Line::from(Span::styled(
            format!("  ↓ {} more", options.len() - window.end),
            theme.dim(),
        )));
    }
    lines.extend([
        Line::from(""),
        Line::from(vec![
            Span::styled("Enter", theme.badge(theme.success)),
            Span::styled(" preview/apply and stay open", theme.dim()),
        ]),
        Line::from(vec![
            Span::styled("Esc/q", theme.badge(theme.text_muted)),
            Span::styled(" close picker", theme.dim()),
        ]),
        Line::from(vec![
            Span::styled("↑/↓", theme.badge(theme.info)),
            Span::styled(" scroll window", theme.dim()),
        ]),
    ]);
    if !app.boxed_chrome {
        lines.insert(
            1,
            Line::from(Span::styled("─".repeat(area.width as usize), theme.dim())),
        );
    }
    let mut p = Paragraph::new(lines).style(theme.panel());
    if app.boxed_chrome {
        p = p.block(
            Block::default()
                .title(" Theme Selector ")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .padding(Padding::horizontal(1))
                .border_style(Style::default().fg(theme.image)),
        );
    }
    frame.render_widget(p, area);
}

fn draw_files(frame: &mut Frame<'_>, app: &App, theme: &Theme, area: Rect) {
    let entries = app.file_entries();
    let visible_rows = area.height.saturating_sub(2) as usize;
    let window = selected_window(entries.len(), app.selected_file, visible_rows);
    let mut items: Vec<ListItem> = Vec::new();
    if window.start > 0 {
        items.push(ListItem::new(Line::from(Span::styled(
            format!("  ↑ {} more", window.start),
            theme.dim(),
        ))));
    }
    items.extend(
        entries
            .iter()
            .enumerate()
            .skip(window.start)
            .take(window.len())
            .map(|(i, e)| {
                let selected = i == app.selected_file;
                let current = app.file_path.as_ref().is_some_and(|path| path == &e.path);
                let marker = if selected { "›" } else { " " };
                let current_marker = if current { " ●" } else { "" };
                let icon = if e.name == "../" {
                    "↰"
                } else if e.is_dir {
                    "📁"
                } else {
                    "󰈙"
                };
                let style = if selected {
                    Style::default()
                        .fg(theme.bg)
                        .bg(theme.accent)
                        .add_modifier(Modifier::BOLD)
                } else if current {
                    Style::default()
                        .fg(theme.success)
                        .add_modifier(Modifier::BOLD)
                } else if e.is_dir {
                    Style::default().fg(theme.info)
                } else {
                    Style::default().fg(theme.text)
                };
                ListItem::new(Line::from(vec![
                    Span::styled(format!("{marker} {icon} {}", e.name), style),
                    Span::styled(current_marker, Style::default().fg(theme.success)),
                ]))
            }),
    );
    if window.end < entries.len() {
        items.push(ListItem::new(Line::from(Span::styled(
            format!("  ↓ {} more", entries.len() - window.end),
            theme.dim(),
        ))));
    }
    let mut list = List::new(items).style(theme.panel());
    if app.boxed_chrome {
        list = list.block(
            Block::default()
                .title(format!(
                    " Files: {} ",
                    compact_path(
                        &app.file_browser_cwd.display().to_string(),
                        area.width as usize
                    )
                ))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .padding(Padding::horizontal(1))
                .border_style(Style::default().fg(theme.border_strong)),
        );
    }
    frame.render_widget(list, area);
}

fn compact_path(path: &str, width: usize) -> String {
    let max = width.saturating_sub(12).max(12);
    if path.chars().count() <= max {
        return path.to_string();
    }
    let tail: String = path
        .chars()
        .rev()
        .take(max.saturating_sub(1))
        .collect::<String>()
        .chars()
        .rev()
        .collect();
    format!("…{tail}")
}

fn draw_help(frame: &mut Frame<'_>, theme: &Theme, area: Rect) {
    let lines = vec![
        Line::from(Span::styled(
            "HELP",
            Style::default().fg(theme.warn).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("Ctrl+X e/p  Edit / preview"),
        Line::from("Ctrl+E/V/F  Enter edit/view/files modes"),
        Line::from("View: o outline, r collapse, t themes, c cursor"),
        Line::from("Edit: s save, w wrap"),
        Line::from("Ctrl+Z/Y    Undo / redo"),
        Line::from("Ctrl+X q    Quit"),
        Line::from("Ctrl+Q      Quit from anywhere"),
        Line::from(""),
        Line::from("Preview: ↑/↓ scroll"),
        Line::from("Edit: arrows/Home/End/Page move, type, Enter, Backspace/Delete"),
        Line::from("Outline: ↑/↓ select, Enter jump, Esc editor"),
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
        Span::styled(scroll_status(app), theme.dim()),
        Span::styled("  │  ", Style::default().fg(theme.border_strong)),
        Span::styled(&app.status, Style::default().fg(theme.text_muted)),
    ]);
    frame.render_widget(Paragraph::new(text).style(theme.elevated()), area);
}

fn scroll_status(app: &App) -> String {
    match app.mode {
        ViewMode::Preview => {
            let max = app.max_preview_scroll().max(1);
            let percent = ((app.preview_scroll * 100) / max).min(100);
            format!("Scroll {}%", percent)
        }
        ViewMode::Edit => format!("Top line {}", app.scroll + 1),
    }
}
