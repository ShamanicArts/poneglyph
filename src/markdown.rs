use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};
use serde::Serialize;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::theme::Theme;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub enum LineKind {
    Heading(u8),
    Blockquote,
    UnorderedList,
    OrderedList,
    CodeBlock,
    Table,
    HorizontalRule,
    Empty,
    Text,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ClassifiedLine {
    pub kind: LineKind,
    pub text: String,
    pub level: Option<u8>,
    pub language: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct OutlineItem {
    pub level: u8,
    pub text: String,
    pub line: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub enum InlineKind {
    Text,
    Image,
    Link,
    Autolink,
    Code,
    BoldItalic,
    Bold,
    Italic,
    Strikethrough,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct InlineSegment {
    pub kind: InlineKind,
    pub text: String,
    pub target: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct InlineLine {
    pub line: usize,
    pub segments: Vec<InlineSegment>,
}

pub fn classify_document(content: &str) -> Vec<ClassifiedLine> {
    let mut in_code = false;
    let mut lang = String::new();
    content
        .split('\n')
        .map(|line| {
            if line.starts_with("```") {
                if in_code {
                    in_code = false;
                    let old = std::mem::take(&mut lang);
                    return ClassifiedLine {
                        kind: LineKind::CodeBlock,
                        text: line.to_string(),
                        level: None,
                        language: if old.is_empty() { None } else { Some(old) },
                    };
                }
                lang = line.trim_start_matches("```").trim().to_string();
                in_code = true;
                return ClassifiedLine {
                    kind: LineKind::CodeBlock,
                    text: line.to_string(),
                    level: None,
                    language: if lang.is_empty() {
                        None
                    } else {
                        Some(lang.clone())
                    },
                };
            }
            if in_code {
                return ClassifiedLine {
                    kind: LineKind::CodeBlock,
                    text: line.to_string(),
                    level: None,
                    language: if lang.is_empty() {
                        None
                    } else {
                        Some(lang.clone())
                    },
                };
            }
            let trimmed = line.trim();
            if trimmed.is_empty() {
                return ClassifiedLine {
                    kind: LineKind::Empty,
                    text: line.to_string(),
                    level: None,
                    language: None,
                };
            }
            if is_hr(trimmed) {
                return ClassifiedLine {
                    kind: LineKind::HorizontalRule,
                    text: line.to_string(),
                    level: None,
                    language: None,
                };
            }
            if is_table_row(line) || is_table_separator(line) {
                return ClassifiedLine {
                    kind: LineKind::Table,
                    text: line.to_string(),
                    level: None,
                    language: None,
                };
            }
            if let Some((level, title)) = heading(line) {
                return ClassifiedLine {
                    kind: LineKind::Heading(level),
                    text: title.to_string(),
                    level: Some(level),
                    language: None,
                };
            }
            if trimmed.starts_with('>') {
                return ClassifiedLine {
                    kind: LineKind::Blockquote,
                    text: line.to_string(),
                    level: None,
                    language: None,
                };
            }
            if unordered(line) {
                return ClassifiedLine {
                    kind: LineKind::UnorderedList,
                    text: line.to_string(),
                    level: None,
                    language: None,
                };
            }
            if ordered(line) {
                return ClassifiedLine {
                    kind: LineKind::OrderedList,
                    text: line.to_string(),
                    level: None,
                    language: None,
                };
            }
            ClassifiedLine {
                kind: LineKind::Text,
                text: line.to_string(),
                level: None,
                language: None,
            }
        })
        .collect()
}

pub fn tokenize_inline_document(content: &str) -> Vec<InlineLine> {
    content
        .split('\n')
        .enumerate()
        .map(|(line, text)| InlineLine {
            line,
            segments: tokenize_inline(text),
        })
        .collect()
}

pub fn outline(content: &str) -> Vec<OutlineItem> {
    content
        .split('\n')
        .enumerate()
        .filter_map(|(line, text)| {
            heading(text).map(|(level, title)| OutlineItem {
                level,
                text: title.to_string(),
                line,
            })
        })
        .collect()
}

fn heading(line: &str) -> Option<(u8, &str)> {
    let bytes = line.as_bytes();
    let mut level = 0usize;
    while level < bytes.len() && bytes[level] == b'#' && level < 6 {
        level += 1;
    }
    if level == 0 {
        return None;
    }
    Some((level as u8, line[level..].trim()))
}

fn is_hr(s: &str) -> bool {
    s.len() >= 3
        && (s.chars().all(|c| c == '-')
            || s.chars().all(|c| c == '*')
            || s.chars().all(|c| c == '_'))
}

fn unordered(line: &str) -> bool {
    let s = line.trim_start();
    matches!(s.as_bytes(), [b'-' | b'*' | b'+', b' ', ..])
}

fn ordered(line: &str) -> bool {
    let s = line.trim_start();
    let Some(dot) = s.find('.') else {
        return false;
    };
    dot > 0
        && s[..dot].chars().all(|c| c.is_ascii_digit())
        && s.as_bytes().get(dot + 1) == Some(&b' ')
}

#[derive(Clone, Copy, Debug)]
struct ListInfo<'a> {
    indent: usize,
    level: usize,
    marker: &'a str,
    content: &'a str,
    ordered: bool,
}

fn list_info(line: &str) -> Option<ListInfo<'_>> {
    let indent = line.chars().take_while(|c| c.is_whitespace()).count();
    let level = indent / 2;
    let s = line.trim_start();
    if matches!(s.as_bytes(), [b'-' | b'*' | b'+', b' ', ..]) {
        return Some(ListInfo {
            indent,
            level,
            marker: &s[..1],
            content: &s[2..],
            ordered: false,
        });
    }
    let dot = s.find('.')?;
    if dot > 0
        && s[..dot].chars().all(|c| c.is_ascii_digit())
        && s.as_bytes().get(dot + 1) == Some(&b' ')
    {
        return Some(ListInfo {
            indent,
            level,
            marker: &s[..dot + 1],
            content: &s[dot + 2..],
            ordered: true,
        });
    }
    None
}

fn is_table_row(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.contains('|') && trimmed.matches('|').count() >= 2 && !is_table_separator(line)
}

fn is_table_separator(line: &str) -> bool {
    let trimmed = line.trim().trim_matches('|').trim();
    !trimmed.is_empty()
        && trimmed.split('|').all(|cell| {
            let cell = cell.trim();
            cell.len() >= 3 && cell.chars().all(|c| matches!(c, '-' | ':' | ' '))
        })
}

fn starts_table_block(lines: &[&str], idx: usize) -> bool {
    idx + 1 < lines.len() && is_table_row(lines[idx]) && is_table_separator(lines[idx + 1])
}

fn table_cells(line: &str) -> Vec<String> {
    line.trim()
        .trim_matches('|')
        .split('|')
        .map(|cell| cell.trim().to_string())
        .collect()
}

fn render_table_block(lines: &[&str], width: usize, theme: &Theme) -> Vec<Line<'static>> {
    let rows: Vec<Vec<String>> = lines
        .iter()
        .filter(|line| !is_table_separator(line))
        .map(|line| table_cells(line))
        .collect();
    if rows.is_empty() {
        return Vec::new();
    }
    let cols = rows.iter().map(Vec::len).max().unwrap_or(0);
    let mut widths = vec![3usize; cols];
    for row in &rows {
        for (idx, cell) in row.iter().enumerate() {
            widths[idx] = widths[idx]
                .max(cell.width())
                .min(width.saturating_div(cols.max(1)).max(6));
        }
    }
    let border_style = Style::default().fg(theme.border_strong);
    let header_style = Style::default()
        .fg(theme.heading2)
        .add_modifier(Modifier::BOLD);
    let text_style = Style::default().fg(theme.text);
    let mut out = Vec::new();
    out.push(table_rule('╭', '┬', '╮', &widths, border_style));
    for (row_idx, row) in rows.iter().enumerate() {
        let style = if row_idx == 0 {
            header_style
        } else {
            text_style
        };
        out.push(table_row(row, &widths, style, border_style));
        if row_idx == 0 && rows.len() > 1 {
            out.push(table_rule('├', '┼', '┤', &widths, border_style));
        }
    }
    out.push(table_rule('╰', '┴', '╯', &widths, border_style));
    out
}

fn table_rule(
    left: char,
    join: char,
    right: char,
    widths: &[usize],
    style: Style,
) -> Line<'static> {
    let mut s = String::new();
    s.push(left);
    for (idx, width) in widths.iter().enumerate() {
        if idx > 0 {
            s.push(join);
        }
        s.push_str(&"─".repeat(*width + 2));
    }
    s.push(right);
    Line::from(Span::styled(s, style))
}

fn table_row(row: &[String], widths: &[usize], style: Style, border_style: Style) -> Line<'static> {
    let mut spans = Vec::new();
    spans.push(Span::styled("│", border_style));
    for (idx, width) in widths.iter().enumerate() {
        let cell = row.get(idx).map(String::as_str).unwrap_or("");
        let pad = width.saturating_sub(cell.width());
        spans.push(Span::raw(" "));
        spans.push(Span::styled(cell.to_string(), style));
        spans.push(Span::raw(" ".repeat(pad + 1)));
        spans.push(Span::styled("│", border_style));
    }
    Line::from(spans)
}

/// Default rows reserved for an image when the caller gives no aspect hint
pub const IMAGE_PREVIEW_ROWS: usize = 16;

/// Where a standalone image sits in the flattened preview line stream
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImagePlacement {
    pub target: String,
    pub start: usize,
    pub rows: u16,
}

/// Some alt and target only when the line's sole content is one image
fn standalone_image(raw: &str) -> Option<(String, String)> {
    let mut image: Option<(String, String)> = None;
    for seg in tokenize_inline(raw.trim()) {
        match seg.kind {
            InlineKind::Image => {
                if image.is_some() {
                    return None;
                }
                image = Some((seg.text, seg.target.unwrap_or_default()));
            }
            InlineKind::Text if seg.text.trim().is_empty() => {}
            _ => return None,
        }
    }
    image
}

pub fn render_preview_document(
    content: &str,
    scroll: usize,
    take: usize,
    width: usize,
    theme: &Theme,
) -> Vec<Line<'static>> {
    let (flat, _) =
        render_preview_document_full(content, width, theme, &mut |_, _| IMAGE_PREVIEW_ROWS);
    flat.into_iter().skip(scroll).take(take).collect()
}

/// Full unscrolled line stream plus image placements for the caller to overlay
pub fn render_preview_document_full(
    content: &str,
    width: usize,
    theme: &Theme,
    image_rows: &mut dyn FnMut(&str, usize) -> usize,
) -> (Vec<Line<'static>>, Vec<ImagePlacement>) {
    let mut in_code = false;
    let mut rendered = Vec::new();
    let mut marks: Vec<(usize, String, u16)> = Vec::new();
    let raw_lines: Vec<&str> = content.split('\n').collect();
    let mut idx = 0;
    while idx < raw_lines.len() {
        let raw = raw_lines[idx];
        if !in_code && starts_table_block(&raw_lines, idx) {
            let start = idx;
            idx += 1;
            while idx < raw_lines.len()
                && (is_table_row(raw_lines[idx]) || is_table_separator(raw_lines[idx]))
            {
                idx += 1;
            }
            rendered.extend(render_table_block(&raw_lines[start..idx], width, theme));
            continue;
        }
        if let Some((level, _)) = heading(raw) {
            if level <= 2
                && rendered
                    .last()
                    .is_some_and(|line: &Line| !plain_line(line).trim().is_empty())
            {
                rendered.push(Line::from(""));
            }
        }
        if raw.starts_with("```") {
            if in_code {
                in_code = false;
                rendered.push(Line::from(vec![
                    Span::styled(
                        "╰─ ",
                        Style::default()
                            .fg(theme.code_block_border)
                            .bg(theme.code_bg),
                    ),
                    Span::styled(
                        raw.to_string(),
                        Style::default()
                            .fg(theme.code_block_border)
                            .bg(theme.code_bg),
                    ),
                ]));
            } else {
                let language = raw.trim_start_matches("```").trim().to_string();
                in_code = true;
                let label = if language.is_empty() {
                    "code"
                } else {
                    &language
                };
                rendered.push(Line::from(vec![
                    Span::styled(
                        "╭─ ",
                        Style::default()
                            .fg(theme.code_block_border)
                            .bg(theme.code_bg),
                    ),
                    Span::styled(
                        label.to_string(),
                        Style::default()
                            .fg(theme.code_block_lang)
                            .bg(theme.code_bg)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]));
            }
        } else if in_code {
            rendered.push(Line::from(Span::styled(
                format!("  {raw}"),
                Style::default().fg(theme.code).bg(theme.code_bg),
            )));
        } else if let Some((level, title)) = heading(raw) {
            rendered.push(render_preview_line(raw, theme));
            let underline_len = title.width().clamp(4, width.max(4));
            let underline_char = match level {
                1 => '━',
                2 => '─',
                3 => '┄',
                _ => '·',
            };
            rendered.push(Line::from(Span::styled(
                underline_char.to_string().repeat(underline_len),
                Style::default().fg(heading_color(level, theme)),
            )));
            if level <= 2 {
                rendered.push(Line::from(""));
            }
        } else if let Some((alt, target)) = standalone_image(raw) {
            // Row 0 is the icon fallback shown if graphics can't draw
            let rows = image_rows(&target, width).max(1);
            marks.push((rendered.len(), target, rows as u16));
            rendered.push(Line::from(Span::styled(
                format!("󰥶 {alt}"),
                Style::default()
                    .fg(theme.image)
                    .add_modifier(Modifier::ITALIC),
            )));
            for _ in 1..rows {
                rendered.push(Line::from(""));
            }
            rendered.push(Line::from(Span::styled(
                format!("  {alt}"),
                Style::default()
                    .fg(theme.text_muted)
                    .add_modifier(Modifier::ITALIC),
            )));
        } else {
            rendered.push(render_preview_line(raw, theme));
        }
        idx += 1;
    }

    // Wrap each line translating image marks into flattened-line space
    let mut flat: Vec<Line<'static>> = Vec::new();
    let mut placements = Vec::new();
    let mut marks = marks.into_iter().peekable();
    for (rendered_idx, line) in rendered.into_iter().enumerate() {
        if marks.peek().is_some_and(|(idx, _, _)| *idx == rendered_idx) {
            let (_, target, rows) = marks.next().unwrap();
            placements.push(ImagePlacement {
                target,
                start: flat.len(),
                rows,
            });
        }
        flat.extend(wrap_styled_line(line, width.max(1)));
    }
    (flat, placements)
}

pub fn render_preview_line(raw: &str, theme: &Theme) -> Line<'static> {
    if let Some((level, title)) = heading(raw) {
        let color = heading_color(level, theme);
        let prefix = if level <= 2 { "" } else { "› " };
        let title_text = if level == 1 {
            title.to_uppercase()
        } else {
            title.to_string()
        };
        let mut spans = if prefix.is_empty() {
            Vec::new()
        } else {
            vec![Span::styled(
                prefix,
                Style::default()
                    .fg(theme.heading_marker)
                    .add_modifier(Modifier::BOLD),
            )]
        };
        spans.extend(render_inline_spans(
            &title_text,
            theme,
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ));
        return Line::from(spans);
    }
    let trimmed = raw.trim_start();
    if trimmed.starts_with('>') {
        let indent_len = raw.len().saturating_sub(trimmed.len());
        let quote_markers = trimmed.chars().take_while(|c| *c == '>').count().max(1);
        let content = trimmed[quote_markers..].trim_start();
        let mut spans = vec![
            Span::raw(" ".repeat(indent_len)),
            Span::styled(
                format!("{} ", "▌".repeat(quote_markers)),
                Style::default().fg(theme.quote_marker),
            ),
        ];
        spans.extend(render_inline_spans(
            content,
            theme,
            Style::default()
                .fg(theme.quote)
                .add_modifier(Modifier::ITALIC),
        ));
        return Line::from(spans);
    }
    if is_hr(trimmed) {
        return Line::from(Span::styled(
            "─".repeat(trimmed.len().max(12)),
            Style::default().fg(theme.hr),
        ));
    }
    if let Some(list) = list_info(raw) {
        let marker = if list.ordered {
            format!("{}{} ", " ".repeat(list.indent), list.marker)
        } else {
            let glyph = match list.level % 3 {
                0 => "•",
                1 => "◦",
                _ => "▪",
            };
            format!("{}{glyph} ", " ".repeat(list.indent))
        };
        let mut spans = vec![Span::styled(marker, Style::default().fg(theme.list_marker))];
        spans.extend(render_inline_spans(
            list.content,
            theme,
            Style::default().fg(theme.text),
        ));
        return Line::from(spans);
    }
    if raw.starts_with("```") {
        return Line::from(Span::styled(
            raw.to_string(),
            Style::default().fg(theme.code).bg(theme.code_bg),
        ));
    }
    render_inline(raw, theme)
}

pub fn render_editor_line(raw: &str, theme: &Theme) -> Line<'static> {
    render_preview_line(raw, theme)
}

fn heading_color(level: u8, theme: &Theme) -> ratatui::style::Color {
    match level {
        1 => theme.heading1,
        2 => theme.heading2,
        3 => theme.heading3,
        4 => theme.heading4,
        5 => theme.heading5,
        _ => theme.heading6,
    }
}

fn render_inline(raw: &str, theme: &Theme) -> Line<'static> {
    let spans = render_inline_spans(raw, theme, Style::default().fg(theme.text));
    if spans.is_empty() {
        Line::from("")
    } else {
        Line::from(spans)
    }
}

fn plain_line(line: &Line<'_>) -> String {
    line.spans
        .iter()
        .map(|span| span.content.as_ref())
        .collect()
}

fn wrap_styled_line(line: Line<'static>, width: usize) -> Vec<Line<'static>> {
    let plain = plain_line(&line);
    if plain.width() <= width || plain.is_empty() {
        return vec![line];
    }

    let continuation = continuation_prefix(&plain);
    let continuation_width = continuation.width();
    let indent_style = Style::default();
    let mut rows = Vec::new();
    let mut current: Vec<Span<'static>> = Vec::new();
    let mut current_width = 0usize;
    let mut first_row = true;

    for span in line.spans {
        let style = span.style;
        for ch in span.content.chars() {
            let ch_width = ch.width().unwrap_or(0).max(1);
            let limit = if first_row {
                width
            } else {
                width.saturating_sub(continuation_width).max(1)
            };
            if current_width > 0 && current_width + ch_width > limit {
                rows.push(Line::from(current));
                first_row = false;
                current = if continuation.is_empty() {
                    Vec::new()
                } else {
                    vec![Span::styled(continuation.clone(), indent_style)]
                };
                current_width = continuation_width;
            }
            current.push(Span::styled(ch.to_string(), style));
            current_width += ch_width;
        }
    }
    rows.push(Line::from(current));
    rows
}

fn continuation_prefix(plain: &str) -> String {
    let trimmed = plain.trim_start();
    let leading = plain.len().saturating_sub(trimmed.len());
    if trimmed.starts_with('▌') || trimmed.starts_with('▸') || trimmed.starts_with('›') {
        return " ".repeat(leading + 2);
    }
    if trimmed.starts_with('•')
        || trimmed.starts_with('◦')
        || trimmed.starts_with('▪')
        || matches!(trimmed.as_bytes(), [b'-' | b'*' | b'+', b' ', ..])
    {
        return " ".repeat(leading + 2);
    }
    if let Some(dot) = trimmed.find(". ") {
        if dot > 0 && trimmed[..dot].chars().all(|c| c.is_ascii_digit()) {
            return " ".repeat(leading + dot + 2);
        }
    }
    String::new()
}

fn render_inline_spans(raw: &str, theme: &Theme, default_style: Style) -> Vec<Span<'static>> {
    tokenize_inline(raw)
        .into_iter()
        .map(|seg| match seg.kind {
            InlineKind::Text => Span::styled(seg.text, default_style),
            InlineKind::Image => Span::styled(
                format!("󰥶 {}", seg.text),
                Style::default()
                    .fg(theme.image)
                    .add_modifier(Modifier::ITALIC),
            ),
            InlineKind::Link => Span::styled(
                format!("{} ↗", seg.text),
                Style::default()
                    .fg(theme.link_text)
                    .add_modifier(Modifier::UNDERLINED),
            ),
            InlineKind::Autolink => Span::styled(
                seg.text,
                Style::default()
                    .fg(theme.link_url)
                    .add_modifier(Modifier::UNDERLINED),
            ),
            InlineKind::Code => {
                Span::styled(seg.text, Style::default().fg(theme.code).bg(theme.code_bg))
            }
            InlineKind::Bold => Span::styled(
                seg.text,
                Style::default().fg(theme.bold).add_modifier(Modifier::BOLD),
            ),
            InlineKind::Italic => Span::styled(
                seg.text,
                Style::default()
                    .fg(theme.italic)
                    .add_modifier(Modifier::ITALIC),
            ),
            InlineKind::BoldItalic => Span::styled(
                seg.text,
                Style::default()
                    .fg(theme.bold_italic)
                    .add_modifier(Modifier::BOLD | Modifier::ITALIC),
            ),
            InlineKind::Strikethrough => Span::styled(
                seg.text,
                Style::default()
                    .fg(theme.strikethrough)
                    .add_modifier(Modifier::CROSSED_OUT),
            ),
        })
        .collect()
}

pub fn tokenize_inline(text: &str) -> Vec<InlineSegment> {
    let mut segments = Vec::new();
    let mut rest = text;
    while !rest.is_empty() {
        if let Some((seg, next)) = match_inline(rest) {
            segments.push(seg);
            rest = next;
            continue;
        }
        let ch = rest.chars().next().unwrap();
        segments.push(InlineSegment {
            kind: InlineKind::Text,
            text: ch.to_string(),
            target: None,
        });
        rest = &rest[ch.len_utf8()..];
    }
    coalesce_text(segments)
}

fn match_inline(rest: &str) -> Option<(InlineSegment, &str)> {
    if let Some(stripped) = rest.strip_prefix("![") {
        if let Some((alt, target, next)) = bracket_target(stripped) {
            return Some((segment(InlineKind::Image, alt, Some(target)), next));
        }
    }
    if let Some(stripped) = rest.strip_prefix('[') {
        if let Some((text, target, next)) = bracket_target(stripped) {
            return Some((segment(InlineKind::Link, text, Some(target)), next));
        }
    }
    if let Some(stripped) = rest.strip_prefix('<') {
        if let Some(end) = stripped.find('>') {
            let target = &stripped[..end];
            if target.contains("://") || target.contains('@') {
                return Some((
                    segment(InlineKind::Autolink, target, Some(target)),
                    &stripped[end + 1..],
                ));
            }
        }
    }
    for (open, close, kind) in [
        ("`", "`", InlineKind::Code),
        ("***", "***", InlineKind::BoldItalic),
        ("___", "___", InlineKind::BoldItalic),
        ("**", "**", InlineKind::Bold),
        ("__", "__", InlineKind::Bold),
        ("*", "*", InlineKind::Italic),
        ("_", "_", InlineKind::Italic),
        ("~~", "~~", InlineKind::Strikethrough),
    ] {
        if let Some(stripped) = rest.strip_prefix(open) {
            if let Some(end) = stripped.find(close) {
                let text = &stripped[..end];
                if !text.is_empty() {
                    return Some((segment(kind, text, None), &stripped[end + close.len()..]));
                }
            }
        }
    }
    None
}

fn bracket_target(stripped_after_open_bracket: &str) -> Option<(&str, &str, &str)> {
    let end_text = stripped_after_open_bracket.find("](")?;
    let text = &stripped_after_open_bracket[..end_text];
    let after = &stripped_after_open_bracket[end_text + 2..];
    let end_target = after.find(')')?;
    Some((text, &after[..end_target], &after[end_target + 1..]))
}

fn segment(kind: InlineKind, text: &str, target: Option<&str>) -> InlineSegment {
    InlineSegment {
        kind,
        text: text.to_string(),
        target: target.map(str::to_string),
    }
}

fn coalesce_text(segments: Vec<InlineSegment>) -> Vec<InlineSegment> {
    let mut out: Vec<InlineSegment> = Vec::new();
    for seg in segments {
        if seg.kind == InlineKind::Text {
            if let Some(prev) = out.last_mut() {
                if prev.kind == InlineKind::Text {
                    prev.text.push_str(&seg.text);
                    continue;
                }
            }
        }
        out.push(seg);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_like_md_editor_core_shapes() {
        let doc = "# H1\n\n> quote\n- item\n1. one\n---\n```ts\nlet x = 1;\n```";
        let kinds: Vec<_> = classify_document(doc).into_iter().map(|l| l.kind).collect();
        assert_eq!(
            kinds,
            vec![
                LineKind::Heading(1),
                LineKind::Empty,
                LineKind::Blockquote,
                LineKind::UnorderedList,
                LineKind::OrderedList,
                LineKind::HorizontalRule,
                LineKind::CodeBlock,
                LineKind::CodeBlock,
                LineKind::CodeBlock
            ]
        );
    }

    #[test]
    fn extracts_outline() {
        let items = outline("# A\nbody\n### C");
        assert_eq!(
            items,
            vec![
                OutlineItem {
                    level: 1,
                    text: "A".into(),
                    line: 0
                },
                OutlineItem {
                    level: 3,
                    text: "C".into(),
                    line: 2
                }
            ]
        );
    }

    #[test]
    fn headings_match_bun_optional_space_behavior() {
        assert_eq!(heading("#NoSpace"), Some((1, "NoSpace")));
        assert_eq!(heading("###### Six"), Some((6, "Six")));
        assert_eq!(heading("####### Seven"), Some((6, "# Seven")));
    }

    #[test]
    fn wraps_preview_rows_with_markdown_continuation_indent() {
        let theme = Theme::slate();
        let rows = render_preview_document(
            "- this is a very long list item that should wrap with indentation",
            0,
            10,
            24,
            &theme,
        );
        let plain: Vec<String> = rows
            .into_iter()
            .map(|line| {
                line.spans
                    .into_iter()
                    .map(|s| s.content.to_string())
                    .collect()
            })
            .collect();
        assert!(plain.len() > 1);
        assert!(plain[1].starts_with("  "));
    }

    #[test]
    fn preview_uses_rich_markdown_shapes_instead_of_raw_hashes() {
        let theme = Theme::slate();
        let rows = render_preview_document("# Title\n\n- item", 0, 10, 40, &theme);
        let plain: Vec<String> = rows
            .into_iter()
            .map(|line| {
                line.spans
                    .into_iter()
                    .map(|s| s.content.to_string())
                    .collect()
            })
            .collect();
        assert!(plain.iter().any(|line| line.contains("TITLE")));
        assert!(plain.iter().any(|line| line.contains("━━━━")));
        assert!(plain.iter().any(|line| line.contains("• item")));
        assert!(plain.iter().filter(|line| line.is_empty()).count() >= 2);
        assert!(!plain.iter().any(|line| line.starts_with("# Title")));
    }

    #[test]
    fn renders_nested_lists_with_depth_markers() {
        let theme = Theme::slate();
        let rows = render_preview_document("- top\n  - nested\n    - deep", 0, 10, 40, &theme);
        let plain: Vec<String> = rows.into_iter().map(|line| plain_line(&line)).collect();
        assert!(plain.iter().any(|line| line.contains("• top")));
        assert!(plain.iter().any(|line| line.contains("  ◦ nested")));
        assert!(plain.iter().any(|line| line.contains("    ▪ deep")));
    }

    #[test]
    fn renders_pipe_tables_as_boxed_preview_rows() {
        let theme = Theme::slate();
        let rows = render_preview_document(
            "| Feature | Status |\n| --- | --- |\n| Tables | done |",
            0,
            10,
            80,
            &theme,
        );
        let plain: Vec<String> = rows.into_iter().map(|line| plain_line(&line)).collect();
        assert!(plain.iter().any(|line| line.starts_with("╭")));
        assert!(plain.iter().any(|line| line.contains("Feature")));
        assert!(plain.iter().any(|line| line.contains("Tables")));
        assert!(plain.iter().any(|line| line.starts_with("╰")));
    }

    #[test]
    fn tokenizes_inline_markdown_shapes() {
        let segs = tokenize_inline(
            "![alt](img.png) [site](https://x.test) `code` ***bi*** **b** _i_ ~~s~~",
        );
        let kinds: Vec<_> = segs.into_iter().map(|s| s.kind).collect();
        assert!(kinds.contains(&InlineKind::Image));
        assert!(kinds.contains(&InlineKind::Link));
        assert!(kinds.contains(&InlineKind::Code));
        assert!(kinds.contains(&InlineKind::BoldItalic));
        assert!(kinds.contains(&InlineKind::Bold));
        assert!(kinds.contains(&InlineKind::Italic));
        assert!(kinds.contains(&InlineKind::Strikethrough));
    }

    #[test]
    fn standalone_image_reserves_a_block_and_reports_placement() {
        let theme = Theme::slate();
        let (lines, placements) = render_preview_document_full(
            "intro\n\n![a cat](cat.png)\n\nmore",
            40,
            &theme,
            &mut |_, _| IMAGE_PREVIEW_ROWS,
        );
        assert_eq!(placements.len(), 1);
        let placement = &placements[0];
        assert_eq!(placement.target, "cat.png");
        assert_eq!(placement.rows as usize, IMAGE_PREVIEW_ROWS);
        assert!(plain_line(&lines[placement.start]).contains("a cat"));
        assert!(lines.len() >= placement.start + IMAGE_PREVIEW_ROWS);
    }

    #[test]
    fn inline_image_mixed_with_text_is_not_treated_as_a_block() {
        let theme = Theme::slate();
        let (_, placements) =
            render_preview_document_full("see ![a cat](cat.png) here", 40, &theme, &mut |_, _| {
                IMAGE_PREVIEW_ROWS
            });
        assert!(placements.is_empty());
    }
}
