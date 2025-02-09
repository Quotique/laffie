use std::{io, path::PathBuf};

use clap::Parser;
use ratatui::{
    crossterm::event::{self, KeyCode, KeyEventKind},
    prelude::*,
    widgets::{Block, Borders, Paragraph, Tabs},
    DefaultTerminal,
};

mod interface;
mod rules;
mod settings;
mod tasks;
mod tracing;

use interface::Tab as Itab;
use settings::Settings;

/// Core develop/debug enviroment
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Sets a custom config file
    #[clap(short, long, default_value = "./config/cli.yaml")]
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
    let mut status = interface::Status::try_new(
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
                "←↑→↓ - navigation | q - quit | s - solve selected | a - solve all | Space - toggle tree node",
            )
            .block(Block::default().borders(Borders::LEFT | Borders::RIGHT));

            frame.render_widget(help, vertical_layout[2]);
        })?;

        if let event::Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                match key.code {
                    KeyCode::F(1) => status.current_tab = Itab::Rules,
                    KeyCode::F(2) => status.current_tab = Itab::Tasks,
                    KeyCode::F(3) => status.current_tab = Itab::Tracing,
                    // KeyCode::F(4) => status.current_tab = Itab::Setting,
                    KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('о') => status.next(),
                    KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('л') => status.previous(),
                    KeyCode::Left | KeyCode::Char('h') | KeyCode::Char('р') => status.left(),
                    KeyCode::Right | KeyCode::Char('l') | KeyCode::Char('д') => status.right(),
                    KeyCode::Enter | KeyCode::Char(' ') => status.toggle(),

                    KeyCode::Char('s') | KeyCode::Char('ы') => status.solve(),
                    KeyCode::Char('a') | KeyCode::Char('ф') => status.solve_all(),

                    KeyCode::Char('q') | KeyCode::Char('й') => return Ok(()),
                    _ => {}
                }
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
