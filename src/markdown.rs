use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};
use serde::Serialize;

use crate::theme::Theme;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub enum LineKind {
    Heading(u8),
    Blockquote,
    UnorderedList,
    OrderedList,
    CodeBlock,
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

pub fn render_preview_line<'a>(raw: &'a str, theme: &Theme) -> Line<'a> {
    if let Some((level, title)) = heading(raw) {
        let color = match level {
            1 => theme.heading1,
            2 => theme.heading2,
            3 => theme.heading3,
            4 => theme.heading4,
            5 => theme.heading5,
            _ => theme.heading6,
        };
        return Line::from(vec![
            Span::styled(
                format!("{} ", "#".repeat(level as usize)),
                Style::default().fg(theme.heading_marker),
            ),
            Span::styled(
                title.to_string(),
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            ),
        ]);
    }
    let trimmed = raw.trim_start();
    if trimmed.starts_with('>') {
        return Line::from(vec![
            Span::styled("▌ ", Style::default().fg(theme.quote_marker)),
            Span::styled(
                trimmed.trim_start_matches('>').trim().to_string(),
                Style::default()
                    .fg(theme.quote)
                    .add_modifier(Modifier::ITALIC),
            ),
        ]);
    }
    if is_hr(trimmed) {
        return Line::from(Span::styled(
            "─".repeat(trimmed.len().max(12)),
            Style::default().fg(theme.hr),
        ));
    }
    if unordered(raw) || ordered(raw) {
        let prefix_len = raw.find(' ').map(|i| i + 1).unwrap_or(0);
        let (prefix, rest) = raw.split_at(prefix_len.min(raw.len()));
        return Line::from(vec![
            Span::styled(prefix.to_string(), Style::default().fg(theme.warn)),
            Span::raw(rest.to_string()),
        ]);
    }
    if raw.starts_with("```") {
        return Line::from(Span::styled(
            raw.to_string(),
            Style::default().fg(theme.code).bg(theme.code_bg),
        ));
    }
    render_inline(raw, theme)
}

pub fn render_editor_line<'a>(raw: &'a str, theme: &Theme) -> Line<'a> {
    render_preview_line(raw, theme)
}

fn render_inline<'a>(raw: &'a str, theme: &Theme) -> Line<'a> {
    let spans: Vec<Span> = tokenize_inline(raw)
        .into_iter()
        .map(|seg| match seg.kind {
            InlineKind::Text => Span::styled(seg.text, Style::default().fg(theme.text)),
            InlineKind::Image => Span::styled(
                format!("![{}]", seg.text),
                Style::default()
                    .fg(theme.image)
                    .add_modifier(Modifier::ITALIC),
            ),
            InlineKind::Link | InlineKind::Autolink => Span::styled(
                seg.text,
                Style::default()
                    .fg(theme.link)
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
        .collect();
    if spans.is_empty() {
        Line::from("")
    } else {
        Line::from(spans)
    }
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
}
