mod app;
mod config;
mod debug_emit;
mod image_view;
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

use app::{App, FocusPane, LeaderMode, ViewMode};
use config::AppConfig;
use theme::Theme;

#[derive(Parser)]
#[command(
    name = "poneglyph",
    about = "A tiny, beautiful terminal markdown editor"
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
    /// Print Rust theme tokens in the Bun md-editor token schema
    ThemeTokens {
        #[arg(long, default_value = "slate")]
        theme: String,
    },
    /// Print effective config preferences JSON
    Config,
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
        Some(Command::ThemeTokens { theme }) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&Theme::named(&theme).tokens())?
            );
            return Ok(());
        }
        Some(Command::Config) => {
            println!("{}", serde_json::to_string_pretty(&AppConfig::load())?);
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
    let result = run_app(&mut terminal, App::new(cli.file)?);
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

fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, mut app: App) -> Result<()> {
    let mut images = image_view::ImageManager::new(image_base_dir(&app), app.allow_remote_images);
    loop {
        images.set_base_dir(image_base_dir(&app));
        terminal.draw(|frame| ui::draw(frame, &app, &app.theme, &mut images))?;
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

fn image_base_dir(app: &App) -> PathBuf {
    app.file_path
        .as_ref()
        .and_then(|p| p.parent())
        .filter(|p| !p.as_os_str().is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

fn parse_script_key(raw: &str) -> Result<KeyEvent> {
    let lower = raw.to_ascii_lowercase();
    let key = match lower.as_str() {
        "esc" | "escape" => KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
        "enter" | "return" => KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
        "backspace" | "bs" => KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE),
        "delete" | "del" => KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE),
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
        "ctrl+e" | "c-e" | "^e" => KeyEvent::new(KeyCode::Char('e'), KeyModifiers::CONTROL),
        "ctrl+v" | "c-v" | "^v" => KeyEvent::new(KeyCode::Char('v'), KeyModifiers::CONTROL),
        "ctrl+f" | "c-f" | "^f" => KeyEvent::new(KeyCode::Char('f'), KeyModifiers::CONTROL),
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
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        match key.code {
            KeyCode::Char('e') => {
                app.enter_leader_mode(LeaderMode::Edit);
                return Ok(());
            }
            KeyCode::Char('v') => {
                app.enter_leader_mode(LeaderMode::View);
                return Ok(());
            }
            KeyCode::Char('f') => {
                app.enter_leader_mode(LeaderMode::Files);
                return Ok(());
            }
            _ => {}
        }
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
        app.status = "Legacy Ctrl+X: e edit, v view, f files, o outline, t themes, c cursor".into();
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

    if app.theme_picker_mode {
        return handle_theme_picker_key(app, key);
    }

    if app.show_help {
        match key.code {
            KeyCode::Esc | KeyCode::Char('h') => app.show_help = false,
            _ => {}
        }
        return Ok(());
    }

    if app.active_leader_mode.is_some() {
        return handle_active_leader_key(app, key);
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

fn handle_active_leader_key(app: &mut App, key: KeyEvent) -> Result<()> {
    if matches!(key.code, KeyCode::Esc | KeyCode::Char('q')) {
        app.exit_active_leader_mode();
        return Ok(());
    }
    match app.active_leader_mode {
        Some(LeaderMode::Edit) => match key.code {
            KeyCode::Char('s') => {
                app.save()?;
                app.active_leader_mode = None;
            }
            KeyCode::Char('w') => {
                app.status = "Wrap toggle: always on for preview".into();
                app.active_leader_mode = None;
            }
            _ => handle_edit_key(app, key)?,
        },
        Some(LeaderMode::View) => match key.code {
            KeyCode::Char('o') => {
                app.show_outline();
                app.active_leader_mode = None;
            }
            KeyCode::Char('r') => {
                app.sidebar_collapsed = !app.sidebar_collapsed;
                app.sidebar_visible = true;
                app.status = if app.sidebar_collapsed {
                    "Sidebar: collapsed"
                } else {
                    "Sidebar: expanded"
                }
                .into();
                app.active_leader_mode = None;
            }
            KeyCode::Char('t') => {
                app.start_theme_picker();
                app.active_leader_mode = None;
            }
            KeyCode::Char('b') => {
                app.toggle_boxed_chrome();
                app.active_leader_mode = None;
            }
            KeyCode::Char('c') => {
                app.cycle_cursor_style();
                app.active_leader_mode = None;
            }
            _ => handle_preview_key(app, key)?,
        },
        Some(LeaderMode::Files) => handle_files_key(app, key)?,
        None => {}
    }
    Ok(())
}

fn handle_theme_picker_key(app: &mut App, key: KeyEvent) -> Result<()> {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => app.cancel_theme_picker(),
        KeyCode::Enter => app.apply_selected_theme_preview(),
        KeyCode::Up | KeyCode::Char('k') => app.move_theme_selection(-1),
        KeyCode::Down | KeyCode::Char('j') => app.move_theme_selection(1),
        _ => {}
    }
    Ok(())
}

fn handle_preview_key(app: &mut App, key: KeyEvent) -> Result<()> {
    match key.code {
        KeyCode::Up | KeyCode::Char('k') => app.scroll_preview(-1),
        KeyCode::Down | KeyCode::Char('j') => app.scroll_preview(1),
        KeyCode::PageUp => app.scroll_preview(-10),
        KeyCode::PageDown => app.scroll_preview(10),
        KeyCode::Home => app.preview_home(),
        KeyCode::End => app.preview_end(),
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
            app.active_leader_mode = None;
            app.status = "View: preview".into();
        }
        KeyCode::Enter => app.newline(),
        KeyCode::Backspace => app.backspace(),
        KeyCode::Delete => app.delete_forward(),
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
        KeyCode::Left => app.file_browser_parent(),
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
        replay(&mut app, &["ctrl+e", "H", "i", "esc"]);
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
        replay(&mut edit, &["ctrl+e", "ctrl+q"]);
        assert!(edit.should_quit);
    }

    #[test]
    fn global_undo_redo_work_while_in_preview() {
        let mut app = App::new(None).unwrap();
        app.content = "abc".into();
        replay(&mut app, &["ctrl+e", "d", "esc", "ctrl+z"]);
        assert_eq!(app.content, "abc");
        replay(&mut app, &["ctrl+y"]);
        assert_eq!(app.content, "dabc");
    }

    #[test]
    fn files_focus_escape_returns_to_editor_focus() {
        let mut app = App::new(None).unwrap();
        replay(&mut app, &["ctrl+f"]);
        assert_eq!(app.focus, FocusPane::Files);
        assert_eq!(app.active_leader_mode, Some(LeaderMode::Files));
        replay(&mut app, &["esc"]);
        assert_eq!(app.focus, FocusPane::Editor);
        assert_eq!(app.active_leader_mode, None);
    }

    #[test]
    fn view_mode_theme_picker_applies_selected_theme() {
        let mut app = App::new(None).unwrap();
        replay(&mut app, &["ctrl+v", "t", "up", "enter"]);
        assert_ne!(app.theme_name, "slate");
        assert!(app.theme_picker_mode);
        assert!(app.status.starts_with("Theme preview -> "));
    }

    #[test]
    fn view_mode_border_command_toggles_boxed_chrome() {
        let mut app = App::new(None).unwrap();
        assert!(app.boxed_chrome);
        replay(&mut app, &["ctrl+v", "b"]);
        assert!(!app.boxed_chrome);
        assert_eq!(app.active_leader_mode, None);
    }

    #[test]
    fn view_mode_cursor_command_cycles_cursor_style() {
        let mut app = App::new(None).unwrap();
        assert_eq!(app.cursor_style, crate::app::CursorStyle::Block);
        replay(&mut app, &["ctrl+v", "c"]);
        assert_eq!(app.cursor_style, crate::app::CursorStyle::Bar);
        assert_eq!(app.active_leader_mode, None);
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
        let path = dir.join(format!("poneglyph-save-test-{}.md", std::process::id()));
        std::fs::write(&path, "abc").unwrap();
        let mut app = App::new(Some(path.clone())).unwrap();
        replay(&mut app, &["ctrl+e", "Z", "ctrl+s"]);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "Zabc");
        assert!(!app.modified);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn preview_scroll_keys_are_bounded_and_support_home_end() {
        let mut app = App::new(None).unwrap();
        app.content = (0..5)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        replay(&mut app, &["pagedown", "pagedown", "end"]);
        assert_eq!(app.preview_scroll, 4);
        replay(&mut app, &["pagedown"]);
        assert_eq!(app.preview_scroll, 4);
        replay(&mut app, &["home"]);
        assert_eq!(app.preview_scroll, 0);
    }

    #[test]
    fn edit_delete_key_removes_forward_character() {
        let mut app = App::new(None).unwrap();
        app.content = "abc".into();
        replay(&mut app, &["ctrl+e", "right", "delete"]);
        assert_eq!(app.content, "ac");
    }

    #[test]
    fn edit_home_end_and_page_keys_move_predictably() {
        let mut app = App::new(None).unwrap();
        app.content = (0..30)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        replay(&mut app, &["ctrl+e", "pagedown", "end"]);
        assert_eq!(app.cursor_line, 10);
        assert_eq!(app.cursor_col, "line 10".len());
        assert!(app.scroll > 0);
        replay(&mut app, &["home", "pageup"]);
        assert_eq!(app.cursor_line, 0);
        assert_eq!(app.cursor_col, 0);
    }
}
