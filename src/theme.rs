use ratatui::style::{Color, Modifier, Style};

#[derive(Clone, Debug)]
pub struct Theme {
    pub bg: Color,
    pub panel: Color,
    pub panel_elevated: Color,
    pub bg2: Color,
    pub border: Color,
    pub border_strong: Color,
    pub text: Color,
    pub text_muted: Color,
    pub info: Color,
    pub accent: Color,
    pub success: Color,
    pub warn: Color,
    #[allow(dead_code)]
    pub error: Color,
    pub heading1: Color,
    pub heading2: Color,
    pub heading3: Color,
    pub heading4: Color,
    pub heading5: Color,
    pub heading6: Color,
    pub heading_marker: Color,
    pub bold: Color,
    pub italic: Color,
    pub bold_italic: Color,
    pub code: Color,
    pub code_bg: Color,
    pub quote: Color,
    pub quote_marker: Color,
    pub link: Color,
    pub image: Color,
    pub strikethrough: Color,
    pub hr: Color,
}

impl Theme {
    pub fn slate() -> Self {
        Self {
            bg: Color::Rgb(0x11, 0x13, 0x18),
            panel: Color::Rgb(0x1a, 0x20, 0x29),
            panel_elevated: Color::Rgb(0x22, 0x2b, 0x37),
            bg2: Color::Rgb(0x1d, 0x25, 0x30),
            border: Color::Rgb(0x2a, 0x34, 0x42),
            border_strong: Color::Rgb(0x3a, 0x47, 0x59),
            text: Color::Rgb(0xd8, 0xde, 0xe8),
            text_muted: Color::Rgb(0x8a, 0x95, 0xa7),
            info: Color::Rgb(0x7f, 0x9f, 0xbf),
            accent: Color::Rgb(0x6f, 0x89, 0xad),
            success: Color::Rgb(0x74, 0xb8, 0x8c),
            warn: Color::Rgb(0xc9, 0xa8, 0x6a),
            error: Color::Rgb(0xce, 0x6f, 0x7c),
            heading1: Color::Rgb(0x4e, 0xc9, 0xb0),
            heading2: Color::Rgb(0x4f, 0xc1, 0xff),
            heading3: Color::Rgb(0x56, 0x9c, 0xd6),
            heading4: Color::Rgb(0x9c, 0xdc, 0xfe),
            heading5: Color::Rgb(0xce, 0x91, 0x78),
            heading6: Color::Rgb(0xb5, 0xce, 0xa8),
            heading_marker: Color::Rgb(0x80, 0x80, 0x80),
            bold: Color::Rgb(0xce, 0x91, 0x78),
            italic: Color::Rgb(0x56, 0x9c, 0xd6),
            bold_italic: Color::Rgb(0xc5, 0x86, 0xc0),
            code: Color::Rgb(0xdc, 0xdc, 0xaa),
            code_bg: Color::Rgb(0x2d, 0x2d, 0x2d),
            quote: Color::Rgb(0x6a, 0x99, 0x55),
            quote_marker: Color::Rgb(0x80, 0x80, 0x80),
            link: Color::Rgb(0x4f, 0xc1, 0xff),
            image: Color::Rgb(0xc5, 0x86, 0xc0),
            strikethrough: Color::Rgb(0x6a, 0x99, 0x55),
            hr: Color::Rgb(0x3a, 0x47, 0x59),
        }
    }

    pub fn base(&self) -> Style {
        Style::default().fg(self.text).bg(self.bg)
    }

    pub fn dim(&self) -> Style {
        self.base().fg(self.text_muted)
    }

    pub fn panel(&self) -> Style {
        Style::default().fg(self.text).bg(self.panel)
    }

    pub fn elevated(&self) -> Style {
        Style::default().fg(self.text).bg(self.panel_elevated)
    }

    pub fn badge(&self, color: Color) -> Style {
        Style::default()
            .fg(self.bg)
            .bg(color)
            .add_modifier(Modifier::BOLD)
    }
}
