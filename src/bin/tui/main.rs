use std::{fmt::Display, io, path::Path, process::Command as ProcessCommand};

use ratatui::{
    DefaultTerminal,
    crossterm::{
        event::{
            self, DisableMouseCapture, EnableMouseCapture, KeyCode, KeyEventKind, KeyModifiers,
            MouseButton, MouseEvent, MouseEventKind,
        },
        execute,
    },
    prelude::*,
    widgets::{Block, Borders, Paragraph, Tabs},
};

mod pane;
mod settings;
mod state;
mod theme;
mod ui;
mod widgets;

use settings::Settings;
use ui::{Command, Tab as Itab};

fn help_bar_item<'b>(k: &'b str, v: impl Display) -> Vec<Span<'b>> {
    vec![
        Span::from("[ "),
        Span::styled(k, Style::default().fg(Color::Red)),
        Span::from(" "),
        Span::from(v.to_string()),
        Span::from(" ]"),
    ]
}

fn run(mut terminal: DefaultTerminal, settings: Settings) -> io::Result<()> {
    let mut ui = ui::Ui::try_new(settings)?;
    let mut tabs_rect = Rect::default();
    let mut body_rect = Rect::default();

    loop {
        ui.tick();
        terminal.draw(|frame| {
            let vertical_layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints(vec![
                    Constraint::Min(1),
                    Constraint::Percentage(100),
                    Constraint::Min(1),
                ])
                .split(frame.area());

            tabs_rect = vertical_layout[0];
            body_rect = vertical_layout[1];

            let tabs = Tabs::new(
                Itab::ALL
                    .iter()
                    .enumerate()
                    .map(|(i, tab)| format!("F{}: {}", i + 1, tab)),
            )
            .select(Some(ui.current_tab.into()))
            .block(Block::default().borders(Borders::LEFT | Borders::RIGHT));
            frame.render_widget(tabs, vertical_layout[0]);

            ui.draw(frame, vertical_layout[1]);

            let help_spans: Vec<Span> = ui
                .key_hints()
                .iter()
                .flat_map(|h| help_bar_item(h.key, h.label))
                .collect();
            let help = Paragraph::new(Line::from(help_spans));

            frame.render_widget(help, vertical_layout[2]);
        })?;

        if ui.has_active_worker() && !event::poll(std::time::Duration::from_millis(100))? {
            continue;
        }
        match event::read()? {
            event::Event::Mouse(mouse) => handle_mouse(&mut ui, mouse, tabs_rect, body_rect),
            event::Event::Key(key) if key.kind == KeyEventKind::Press => {
                let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
                let command = if ui.is_filter_mode() {
                    match key.code {
                        KeyCode::Esc => Command::Dismiss,
                        KeyCode::Enter => Command::FilterFinish,
                        KeyCode::Backspace => Command::FilterBackspace,
                        KeyCode::Up => Command::Up,
                        KeyCode::Down => Command::Down,
                        KeyCode::PageUp => Command::PageUp,
                        KeyCode::PageDown => Command::PageDown,
                        KeyCode::Home => Command::Top,
                        KeyCode::End => Command::Bottom,
                        KeyCode::Char(c) if !ctrl => Command::FilterChar(c),
                        _ => Command::None,
                    }
                } else {
                    match key.code {
                        KeyCode::Char('u') | KeyCode::Char('г') if ctrl => Command::PageUp,
                        KeyCode::Char('d') | KeyCode::Char('в') if ctrl => Command::PageDown,
                        KeyCode::F(1) => Command::SwitchTab(0),
                        KeyCode::F(2) => Command::SwitchTab(1),
                        KeyCode::F(3) => Command::SwitchTab(2),
                        // KeyCode::F(4) => status.current_tab = Itab::Setting,
                        KeyCode::PageDown => Command::PageDown,
                        KeyCode::PageUp => Command::PageUp,
                        KeyCode::Home => Command::Top,
                        KeyCode::End => Command::Bottom,
                        KeyCode::Tab => Command::NextPane,
                        KeyCode::BackTab => Command::PrevPane,
                        KeyCode::Esc => Command::Dismiss,
                        KeyCode::Char('?') => Command::ShowHelp,
                        KeyCode::Char('/') => Command::FilterEnter,
                        KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('о') => Command::Down,
                        KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('л') => Command::Up,
                        KeyCode::Left | KeyCode::Char('h') | KeyCode::Char('р') => Command::Left,
                        KeyCode::Right | KeyCode::Char('l') | KeyCode::Char('д') => Command::Right,
                        KeyCode::Enter | KeyCode::Char(' ') => Command::Toggle,
                        KeyCode::Char('s') | KeyCode::Char('ы') => Command::Solve,
                        KeyCode::Char('a') | KeyCode::Char('ф') => Command::SolveAll,
                        KeyCode::Char('r') | KeyCode::Char('к') => Command::Reload,
                        KeyCode::Char('R') | KeyCode::Char('К') => Command::ReloadAll,
                        KeyCode::Char('e') | KeyCode::Char('у') => Command::EditSelected,
                        KeyCode::Char('c') | KeyCode::Char('с') => Command::Cancel,
                        KeyCode::Char('q') | KeyCode::Char('й') => return Ok(()),
                        _ => Command::None,
                    }
                };
                ui.process(command);
            }
            _ => {}
        }

        if let Some(path) = ui.take_pending_edit() {
            run_editor(&mut terminal, &path)?;
            ui.process(Command::ReloadAll);
        }
    }
}

fn run_editor(terminal: &mut DefaultTerminal, path: &Path) -> io::Result<()> {
    let editor = std::env::var("EDITOR")
        .or_else(|_| std::env::var("VISUAL"))
        .unwrap_or_else(|_| "vi".to_string());

    let _ = execute!(io::stdout(), DisableMouseCapture);
    ratatui::restore();
    let _ = ProcessCommand::new(&editor).arg(path).status();
    *terminal = ratatui::init();
    let _ = execute!(io::stdout(), EnableMouseCapture);
    terminal.clear()?;
    Ok(())
}

fn handle_mouse(ui: &mut ui::Ui, mouse: MouseEvent, tabs: Rect, body: Rect) {
    let pos = Position::new(mouse.column, mouse.row);
    match mouse.kind {
        MouseEventKind::ScrollUp => ui.process(Command::Up),
        MouseEventKind::ScrollDown => ui.process(Command::Down),
        MouseEventKind::Down(MouseButton::Left) => {
            if tabs.contains(pos) {
                let count = Itab::ALL.len().max(1);
                let rel_x = mouse.column.saturating_sub(tabs.x) as usize;
                let width = (tabs.width as usize).max(1);
                let idx = (rel_x * count / width).min(count - 1);
                ui.process(Command::SwitchTab(idx));
            } else if body.contains(pos) {
                ui.click_in_body(mouse.column, mouse.row, body);
            }
        }
        _ => {}
    }
}

fn main() -> io::Result<()> {
    let settings = Settings::new()
        .map_err(|e| {
            println!("Config error: {e:?}");
            e
        })
        .unwrap_or_else(|_| {
            std::process::exit(-1);
        });
    let _log_guard = settings.logger.init();

    let mut terminal = ratatui::init();
    let _ = execute!(io::stdout(), EnableMouseCapture);
    terminal.clear()?;
    let app_result = run(terminal, settings);
    let _ = execute!(io::stdout(), DisableMouseCapture);
    ratatui::restore();
    if let Err(e) = app_result {
        eprintln!("{e}");
    }

    Ok(())
}
