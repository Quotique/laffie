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
                        KeyCode::F(4) => Command::SwitchTab(3),
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
                if let Some(idx) = tab_index_for_click(tabs, mouse.column) {
                    ui.process(Command::SwitchTab(idx));
                }
            } else if body.contains(pos) {
                ui.click_in_body(mouse.column, mouse.row, body);
            }
        }
        _ => {}
    }
}

// Tabs widget renders inside Borders::LEFT|RIGHT as
// " title0 │ title1 │ title2 " — leading padding, then each title with one
// trailing pad + divider + leading pad before the next. Returns the index
// whose title text covers `click_col`; None if the click landed on padding
// or a divider.
fn tab_index_for_click(tabs: Rect, click_col: u16) -> Option<usize> {
    let inner_start = tabs.x.saturating_add(1);
    let inner_end = tabs.x.saturating_add(tabs.width).saturating_sub(1);
    if click_col < inner_start || click_col >= inner_end {
        return None;
    }
    let rel = (click_col - inner_start) as usize;
    let mut cursor: usize = 1;
    for (idx, tab) in Itab::ALL.iter().enumerate() {
        let len = format!("F{}: {}", idx + 1, tab).chars().count();
        if rel >= cursor && rel < cursor + len {
            return Some(idx);
        }
        cursor += len + 3;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: u16, w: u16) -> Rect {
        Rect {
            x,
            y: 0,
            width: w,
            height: 1,
        }
    }

    #[test]
    fn clicks_on_each_title_match_index() {
        let tabs = rect(0, 80);
        // Inner starts at 1; titles "F1: Rules" (9), "F2: Tasks" (9),
        // "F3: Tracing" (11), "F4: Settings" (12). Layout cols:
        // pad@1, F1@2..10, pad@11, div@12, pad@13, F2@14..22, pad@23,
        // div@24, pad@25, F3@26..36, pad@37, div@38, pad@39, F4@40..51
        assert_eq!(tab_index_for_click(tabs, 2), Some(0));
        assert_eq!(tab_index_for_click(tabs, 10), Some(0));
        assert_eq!(tab_index_for_click(tabs, 14), Some(1));
        assert_eq!(tab_index_for_click(tabs, 22), Some(1));
        assert_eq!(tab_index_for_click(tabs, 26), Some(2));
        assert_eq!(tab_index_for_click(tabs, 36), Some(2));
        assert_eq!(tab_index_for_click(tabs, 40), Some(3));
        assert_eq!(tab_index_for_click(tabs, 51), Some(3));
    }

    #[test]
    fn clicks_on_padding_or_divider_return_none() {
        let tabs = rect(0, 80);
        assert_eq!(tab_index_for_click(tabs, 1), None); // leading padding
        assert_eq!(tab_index_for_click(tabs, 11), None); // padding after F1
        assert_eq!(tab_index_for_click(tabs, 12), None); // divider
        assert_eq!(tab_index_for_click(tabs, 13), None); // padding before F2
        assert_eq!(tab_index_for_click(tabs, 38), None); // divider before F4
    }

    #[test]
    fn clicks_outside_rect_return_none() {
        let tabs = rect(5, 50);
        assert_eq!(tab_index_for_click(tabs, 0), None);
        assert_eq!(tab_index_for_click(tabs, 4), None);
        assert_eq!(tab_index_for_click(tabs, 54), None);
    }

    #[test]
    fn offset_rect_shifts_boundaries() {
        let tabs = rect(10, 80);
        // Same offsets as the first test, plus 10.
        assert_eq!(tab_index_for_click(tabs, 12), Some(0));
        assert_eq!(tab_index_for_click(tabs, 24), Some(1));
        assert_eq!(tab_index_for_click(tabs, 36), Some(2));
        assert_eq!(tab_index_for_click(tabs, 50), Some(3));
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
