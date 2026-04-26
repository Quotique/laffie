use std::{fmt::Display, io};

use ratatui::{
    DefaultTerminal,
    crossterm::event::{self, KeyCode, KeyEventKind},
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
        if let event::Event::Key(key) = event::read()? &&
            key.kind == KeyEventKind::Press
        {
            let command = match key.code {
                KeyCode::F(1) => Command::SwitchTab(0),
                KeyCode::F(2) => Command::SwitchTab(1),
                KeyCode::F(3) => Command::SwitchTab(2),
                // KeyCode::F(4) => status.current_tab = Itab::Setting,
                KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('о') => Command::Down,

                KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('л') => Command::Up,
                KeyCode::Left | KeyCode::Char('h') | KeyCode::Char('р') => Command::Left,
                KeyCode::Right | KeyCode::Char('l') | KeyCode::Char('д') => Command::Right,
                KeyCode::Enter | KeyCode::Char(' ') => Command::Toggle,
                KeyCode::Char('s') | KeyCode::Char('ы') => Command::Solve,
                KeyCode::Char('a') | KeyCode::Char('ф') => Command::SolveAll,
                KeyCode::Char('r') | KeyCode::Char('к') => Command::Reload,
                KeyCode::Char('c') | KeyCode::Char('с') => Command::Cancel,
                KeyCode::Char('q') | KeyCode::Char('й') => return Ok(()),
                _ => Command::None,
            };
            ui.process(command);
        }
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
    terminal.clear()?;
    let app_result = run(terminal, settings);
    ratatui::restore();
    if let Err(e) = app_result {
        eprintln!("{e}");
    }

    Ok(())
}
