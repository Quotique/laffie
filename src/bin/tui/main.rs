use std::{io, path::PathBuf};

use clap::Parser;
use ratatui::{
    crossterm::event::{self, KeyCode, KeyEventKind},
    prelude::*,
    widgets::{Block, Borders, Paragraph, Tabs},
    DefaultTerminal,
};

mod popup;
mod rules;
mod settings;
mod state;
mod tasks;
mod theme;
mod widgets;

use settings::Settings;
use state::{Command, Tab as Itab};

/// Core develop/debug enviroment
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Sets a custom config file
    #[clap(short, long, default_value = "./config/tui.yaml")]
    config: PathBuf,

    /// Specify symbols path
    #[clap(short, long)]
    symbols: Option<PathBuf>,

    /// Specify tasks path
    #[clap(short = 'p', long)]
    tasks: Option<PathBuf>,

    /// Specify tasks DB path
    #[clap(short = 'd', long, default_value = "./db/tasks")]
    tasks_db: PathBuf,

    /// Execution deadline (in cycles) for individual problem
    #[clap(short, long, default_value = "100000")]
    exec_deadline: usize,
}

fn run(mut terminal: DefaultTerminal, args: &Args) -> io::Result<()> {
    let mut status = state::State::try_new(
        args.exec_deadline,
        args.symbols.clone().unwrap_or("symbols".into()),
        args.tasks.clone().unwrap_or("tasks".into()),
    )?;

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
    let args = Args::parse();

    let settings = Settings::new(args.config.clone())
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
    let app_result = run(terminal, &args);
    ratatui::restore();
    if let Err(e) = app_result {
        eprintln!("{e}");
    }

    Ok(())
}
