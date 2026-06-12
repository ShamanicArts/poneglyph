mod app;
mod debug_emit;
mod markdown;
mod theme;
mod ui;

use std::{io, path::PathBuf, time::Duration};

use anyhow::Result;
use clap::{Parser, Subcommand};
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use app::{App, FocusPane, ViewMode};
use theme::Theme;

#[derive(Parser)]
#[command(
    name = "md-editor-rust",
    about = "Rust/Ratatui parity port spike of md-editor"
)]
struct Cli {
    #[arg(value_name = "FILE")]
    file: Option<PathBuf>,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Print markdown outline JSON for oracle tests
    Outline { file: PathBuf },
    /// Print document stats JSON for oracle tests
    Stats { file: PathBuf },
    /// Print markdown classification JSON for oracle tests
    Classify { file: PathBuf },
    /// Print inline markdown segment JSON for oracle tests
    Inline { file: PathBuf },
    /// Print wrapped preview viewport lines JSON for oracle tests
    PreviewLines {
        file: PathBuf,
        #[arg(long, default_value_t = 80)]
        width: usize,
        #[arg(long, default_value_t = 24)]
        height: usize,
    },
    /// Print wrapped editor viewport lines JSON for oracle tests
    EditorLines {
        file: PathBuf,
        #[arg(long, default_value_t = 80)]
        width: usize,
        #[arg(long, default_value_t = 24)]
        height: usize,
    },
    /// Print sidebar viewport lines JSON for oracle tests
    SidebarLines {
        file: PathBuf,
        #[arg(long, default_value_t = 40)]
        width: usize,
        #[arg(long, default_value_t = 24)]
        height: usize,
        #[arg(long)]
        files: bool,
        #[arg(long = "show-help")]
        show_help: bool,
        #[arg(long)]
        collapsed: bool,
    },
    /// Replay comma-separated key names and print final state JSON
    StateAfterKeys { file: PathBuf, keys: String },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Some(Command::Outline { file }) => {
            let content = std::fs::read_to_string(file)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&markdown::outline(&content))?
            );
            return Ok(());
        }
        Some(Command::Stats { file }) => {
            let mut app = App::new(None)?;
            app.content = std::fs::read_to_string(file)?;
            println!("{}", serde_json::to_string_pretty(&app.stats())?);
            return Ok(());
        }
        Some(Command::Classify { file }) => {
            let content = std::fs::read_to_string(file)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&markdown::classify_document(&content))?
            );
            return Ok(());
        }
        Some(Command::Inline { file }) => {
            let content = std::fs::read_to_string(file)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&markdown::tokenize_inline_document(&content))?
            );
            return Ok(());
        }
        Some(Command::PreviewLines {
            file,
            width,
            height,
        }) => {
            let app = App::new(Some(file))?;
            println!(
                "{}",
                serde_json::to_string_pretty(&debug_emit::preview_lines(&app, width, height))?
            );
            return Ok(());
        }
        Some(Command::EditorLines {
            file,
            width,
            height,
        }) => {
            let mut app = App::new(Some(file))?;
            app.mode = ViewMode::Edit;
            println!(
                "{}",
                serde_json::to_string_pretty(&debug_emit::editor_lines(&app, width, height))?
            );
            return Ok(());
        }
        Some(Command::SidebarLines {
            file,
            width,
            height,
            files,
            show_help,
            collapsed,
        }) => {
            let mut app = App::new(Some(file))?;
            app.sidebar_files = files;
            app.show_help = show_help;
            app.sidebar_collapsed = collapsed;
            println!(
                "{}",
                serde_json::to_string_pretty(&debug_emit::sidebar_lines(&app, width, height))?
            );
            return Ok(());
        }
        Some(Command::StateAfterKeys { file, keys }) => {
            let mut app = App::new(Some(file))?;
            for key in keys.split(',').map(str::trim).filter(|k| !k.is_empty()) {
                let event = parse_script_key(key)?;
                handle_key(&mut app, event)?;
                if app.should_quit {
                    break;
                }
            }
            println!(
                "{}",
                serde_json::to_string_pretty(&debug_emit::state(&app))?
            );
            return Ok(());
        }
        None => {}
    }

    let mut terminal = setup_terminal()?;
    let result = run_app(&mut terminal, App::new(cli.file)?, Theme::slate());
    restore_terminal(&mut terminal)?;
    result
}

fn setup_terminal() -> Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    Ok(Terminal::new(backend)?)
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    mut app: App,
    theme: Theme,
) -> Result<()> {
    loop {
        terminal.draw(|frame| ui::draw(frame, &app, &theme))?;
        if app.should_quit {
            return Ok(());
        }
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    handle_key(&mut app, key)?;
                }
            }
        }
    }
}

fn parse_script_key(raw: &str) -> Result<KeyEvent> {
    let lower = raw.to_ascii_lowercase();
    let key = match lower.as_str() {
        "esc" | "escape" => KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
        "enter" | "return" => KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
        "backspace" | "bs" => KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE),
        "tab" => KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE),
        "up" => KeyEvent::new(KeyCode::Up, KeyModifiers::NONE),
        "down" => KeyEvent::new(KeyCode::Down, KeyModifiers::NONE),
        "left" => KeyEvent::new(KeyCode::Left, KeyModifiers::NONE),
        "right" => KeyEvent::new(KeyCode::Right, KeyModifiers::NONE),
        "pageup" | "pgup" => KeyEvent::new(KeyCode::PageUp, KeyModifiers::NONE),
        "pagedown" | "pgdn" => KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE),
        "home" => KeyEvent::new(KeyCode::Home, KeyModifiers::NONE),
        "end" => KeyEvent::new(KeyCode::End, KeyModifiers::NONE),
        "ctrl+x" | "c-x" | "^x" => KeyEvent::new(KeyCode::Char('x'), KeyModifiers::CONTROL),
        "ctrl+q" | "c-q" | "^q" => KeyEvent::new(KeyCode::Char('q'), KeyModifiers::CONTROL),
        "ctrl+s" | "c-s" | "^s" => KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL),
        "ctrl+z" | "c-z" | "^z" => KeyEvent::new(KeyCode::Char('z'), KeyModifiers::CONTROL),
        "ctrl+y" | "c-y" | "^y" => KeyEvent::new(KeyCode::Char('y'), KeyModifiers::CONTROL),
        _ if raw.chars().count() == 1 => KeyEvent::new(
            KeyCode::Char(raw.chars().next().unwrap()),
            KeyModifiers::NONE,
        ),
        _ => anyhow::bail!("unknown scripted key: {raw}"),
    };
    Ok(key)
}

fn handle_key(app: &mut App, key: KeyEvent) -> Result<()> {
    if key.modifiers.contains(KeyModifiers::CONTROL) && matches!(key.code, KeyCode::Char('c' | 'q'))
    {
        app.should_quit = true;
        return Ok(());
    }
    if key.modifiers.contains(KeyModifiers::CONTROL) && matches!(key.code, KeyCode::Char('s')) {
        app.save()?;
        return Ok(());
    }
    if key.modifiers.contains(KeyModifiers::CONTROL) && matches!(key.code, KeyCode::Char('z')) {
        app.undo();
        return Ok(());
    }
    if key.modifiers.contains(KeyModifiers::CONTROL) && matches!(key.code, KeyCode::Char('y')) {
        app.redo();
        return Ok(());
    }
    if key.modifiers.contains(KeyModifiers::CONTROL) && matches!(key.code, KeyCode::Char('x')) {
        app.leader = true;
        app.status = "Ctrl+X: e edit, p preview, f files, o outline, b/r sidebar, u undo, y redo, s save, q quit".into();
        return Ok(());
    }
    if app.leader {
        match key.code {
            KeyCode::Esc => {
                app.leader = false;
                app.status = "Cancelled".into();
            }
            KeyCode::Char(ch) => app.command(ch)?,
            _ => {}
        }
        return Ok(());
    }

    if app.show_help {
        match key.code {
            KeyCode::Esc | KeyCode::Char('h') => app.show_help = false,
            _ => {}
        }
        return Ok(());
    }

    if matches!(app.focus, FocusPane::Files) {
        return handle_files_key(app, key);
    }
    if matches!(app.focus, FocusPane::Outline) {
        return handle_outline_key(app, key);
    }

    match app.mode {
        ViewMode::Preview => handle_preview_key(app, key),
        ViewMode::Edit => handle_edit_key(app, key),
    }
}

fn handle_preview_key(app: &mut App, key: KeyEvent) -> Result<()> {
    match key.code {
        KeyCode::Up | KeyCode::Char('k') => app.scroll_preview(-1),
        KeyCode::Down | KeyCode::Char('j') => app.scroll_preview(1),
        KeyCode::PageUp => app.scroll_preview(-10),
        KeyCode::PageDown => app.scroll_preview(10),
        KeyCode::Esc => app.focus = FocusPane::Editor,
        _ => {}
    }
    Ok(())
}

fn handle_edit_key(app: &mut App, key: KeyEvent) -> Result<()> {
    match key.code {
        KeyCode::Up => app.move_cursor(-1, 0),
        KeyCode::Down => app.move_cursor(1, 0),
        KeyCode::Left => app.move_cursor(0, -1),
        KeyCode::Right => app.move_cursor(0, 1),
        KeyCode::Home => app.line_home(),
        KeyCode::End => app.line_end(),
        KeyCode::PageUp => app.move_page(-1),
        KeyCode::PageDown => app.move_page(1),
        KeyCode::Esc => {
            app.mode = ViewMode::Preview;
            app.status = "View: preview".into();
        }
        KeyCode::Enter => app.newline(),
        KeyCode::Backspace => app.backspace(),
        KeyCode::Tab => {
            app.insert_char(' ');
            app.insert_char(' ');
        }
        KeyCode::Char(ch) if key.modifiers.is_empty() || key.modifiers == KeyModifiers::SHIFT => {
            app.insert_char(ch)
        }
        _ => {}
    }
    Ok(())
}

fn handle_outline_key(app: &mut App, key: KeyEvent) -> Result<()> {
    let len = app.outline().len().max(1);
    match key.code {
        KeyCode::Up | KeyCode::Char('k') => {
            app.selected_outline = app.selected_outline.saturating_sub(1)
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.selected_outline = (app.selected_outline + 1).min(len - 1)
        }
        KeyCode::Enter | KeyCode::Right => app.jump_to_selected_outline(),
        KeyCode::Esc | KeyCode::Left => app.focus = FocusPane::Editor,
        _ => {}
    }
    Ok(())
}

fn handle_files_key(app: &mut App, key: KeyEvent) -> Result<()> {
    let len = app.file_entries().len().max(1);
    match key.code {
        KeyCode::Up | KeyCode::Char('k') => app.selected_file = app.selected_file.saturating_sub(1),
        KeyCode::Down | KeyCode::Char('j') => {
            app.selected_file = (app.selected_file + 1).min(len - 1)
        }
        KeyCode::Enter | KeyCode::Right => app.open_selected_file()?,
        KeyCode::Left => {
            if let Some(parent) = app.file_browser_cwd.parent() {
                app.file_browser_cwd = parent.to_path_buf();
                app.selected_file = 0;
            }
        }
        KeyCode::Esc => app.focus = FocusPane::Editor,
        _ => {}
    }
    Ok(())
}

#[cfg(test)]
mod input_tests {
    use super::*;

    fn key(raw: &str) -> KeyEvent {
        parse_script_key(raw).unwrap()
    }

    fn replay(app: &mut App, keys: &[&str]) {
        for raw in keys {
            handle_key(app, key(raw)).unwrap();
        }
    }

    #[test]
    fn leader_edit_type_escape_returns_to_preview_without_leaking_command_chars() {
        let mut app = App::new(None).unwrap();
        app.content = "abc".into();
        replay(&mut app, &["ctrl+x", "e", "H", "i", "esc"]);
        assert_eq!(app.mode, ViewMode::Preview);
        assert_eq!(app.content, "Hiabc");
        assert!(!app.leader);
        assert!(!app.content.contains("xe"));
    }

    #[test]
    fn quit_commands_work_from_preview_and_edit_modes() {
        let mut preview = App::new(None).unwrap();
        replay(&mut preview, &["ctrl+q"]);
        assert!(preview.should_quit);

        let mut edit = App::new(None).unwrap();
        replay(&mut edit, &["ctrl+x", "e", "ctrl+x", "q"]);
        assert!(edit.should_quit);
    }

    #[test]
    fn global_undo_redo_work_while_in_preview() {
        let mut app = App::new(None).unwrap();
        app.content = "abc".into();
        replay(&mut app, &["ctrl+x", "e", "d", "esc", "ctrl+z"]);
        assert_eq!(app.content, "abc");
        replay(&mut app, &["ctrl+y"]);
        assert_eq!(app.content, "dabc");
    }

    #[test]
    fn files_focus_escape_returns_to_editor_focus() {
        let mut app = App::new(None).unwrap();
        replay(&mut app, &["ctrl+x", "f"]);
        assert_eq!(app.focus, FocusPane::Files);
        replay(&mut app, &["esc"]);
        assert_eq!(app.focus, FocusPane::Editor);
    }

    #[test]
    fn outline_focus_can_jump_to_heading() {
        let mut app = App::new(None).unwrap();
        app.content = "# A\nbody\n## B\ntext".into();
        replay(&mut app, &["ctrl+x", "o", "down", "enter"]);
        assert_eq!(app.focus, FocusPane::Editor);
        assert_eq!(app.cursor_line, 2);
        assert_eq!(app.preview_scroll, 0);
        assert_eq!(app.status, "Jumped to B");
    }

    #[test]
    fn ctrl_s_saves_from_edit_mode() {
        let dir = std::env::temp_dir();
        let path = dir.join(format!(
            "md-editor-rust-save-test-{}.md",
            std::process::id()
        ));
        std::fs::write(&path, "abc").unwrap();
        let mut app = App::new(Some(path.clone())).unwrap();
        replay(&mut app, &["ctrl+x", "e", "Z", "ctrl+s"]);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "Zabc");
        assert!(!app.modified);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn edit_home_end_and_page_keys_move_predictably() {
        let mut app = App::new(None).unwrap();
        app.content = (0..30)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        replay(&mut app, &["ctrl+x", "e", "pagedown", "end"]);
        assert_eq!(app.cursor_line, 10);
        assert_eq!(app.cursor_col, "line 10".len());
        assert!(app.scroll > 0);
        replay(&mut app, &["home", "pageup"]);
        assert_eq!(app.cursor_line, 0);
        assert_eq!(app.cursor_col, 0);
    }
}
