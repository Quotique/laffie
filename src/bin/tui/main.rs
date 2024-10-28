use std::io;

use ratatui::{
    crossterm::event::{self, KeyCode, KeyEventKind},
    prelude::*,
    widgets::Tabs,
    DefaultTerminal,
};

mod interface;

use interface::Tab as Itab;

fn run(mut terminal: DefaultTerminal) -> io::Result<()> {
    let mut status = interface::Status::new();

    loop {
        terminal.draw(|frame| {
            let vertical_layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints(vec![
                    Constraint::Percentage(5),
                    Constraint::Percentage(90),
                    Constraint::Percentage(5),
                ])
                .split(frame.area());

            let tabs = Tabs::new((0..=Itab::MAX).map(|x| format!("F{}: {}", x + 1, Itab::from(x))))
                .select(status.current_tab.into());

            frame.render_widget(tabs, vertical_layout[0]);

            status.draw(frame, vertical_layout[1]);
        })?;

        if let event::Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                match key.code {
                    KeyCode::F(1) => status.current_tab = Itab::Rules,
                    KeyCode::F(2) => status.current_tab = Itab::Tasks,
                    KeyCode::F(3) => status.current_tab = Itab::Tracing,
                    KeyCode::F(4) => status.current_tab = Itab::Setting,

                    KeyCode::Down | KeyCode::Char('j') => status.next(),
                    KeyCode::Up | KeyCode::Char('k') => status.previous(),
                    KeyCode::Left | KeyCode::Char('h') => status.left(),
                    KeyCode::Right | KeyCode::Char('l') => status.right(),

                    KeyCode::Char('s') => status.solve(),

                    KeyCode::Char('q') => return Ok(()),
                    _ => {}
                }
            }
        }
    }
}

fn main() -> io::Result<()> {
    let mut terminal = ratatui::init();
    terminal.clear()?;
    let app_result = run(terminal);
    ratatui::restore();
    app_result
}
