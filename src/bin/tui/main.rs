use std::io;

use ratatui::{
    crossterm::event::{self, KeyCode, KeyEventKind},
    prelude::*,
    widgets::{Block, Borders, Paragraph, Tabs},
    DefaultTerminal,
};

mod pane;
mod popup;
mod settings;
mod state;
mod theme;
mod ui;
mod widgets;

use settings::Settings;
use ui::{Command, Tab as Itab};

fn run(mut terminal: DefaultTerminal, settings: Settings) -> io::Result<()> {
    let mut status = ui::Ui::try_new(settings)?;

    loop {
        terminal.draw(|frame| {
            let vertical_layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints(vec![
                    Constraint::Min(1),
                    Constraint::Percentage(100),
                    Constraint::Min(1),
                ])
                .split(frame.area());

            let tabs = Tabs::new((0..=Itab::MAX).map(|x| format!("F{}: {}", x + 1, Itab::from(x))))
                .select(Some(status.current_tab.into()))
                .block(Block::default().borders(Borders::LEFT | Borders::RIGHT));
            frame.render_widget(tabs, vertical_layout[0]);

            status.draw(frame, vertical_layout[1]);

            let help = Paragraph::new(
                "←↑→↓ - navigation | q - quit | s - solve selected | r - reload symbols | a - solve all | Space - toggle tree node",
            )
            .block(Block::default().borders(Borders::LEFT | Borders::RIGHT));

            frame.render_widget(help, vertical_layout[2]);
        })?;

        if let event::Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
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
                    KeyCode::Char('q') | KeyCode::Char('й') => return Ok(()),
                    _ => Command::None,
                };
                status.process(command);
            }
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
