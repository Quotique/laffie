use std::{convert::TryFrom, fmt::Write, sync::Arc};

use telegram_bot::*;

use database::{TaskDb, TaskRecord, UserDb, UserRecord};
use parser::{TaskParser, lang};
use solver::{
    rule::RulesEngine,
    task::{EXECUTION_DEADLINE_DEFAULT, SolutionStatus, Solver},
};
use view::{Html, View};

use super::Command;
use crate::pagination::Paginator;

fn task(
    task_text: String,
    engine: Arc<RulesEngine>,
    tasks: Arc<TaskDb>,
    user: &mut UserRecord,
) -> Result<Paginator, String> {
    let states = lang::task(&task_text)
        .map_err(|e| format!("<code>{}</code>", e.error_string(&task_text, None)))?;
    let task = TaskParser::with(&states)
        .parse()
        .map_err(|e| e.to_string())?;

    let record = TaskRecord::from(&task);
    let mut record = tasks.get_or_insert(record).map_err(|e| e.to_string())?;

    let mut solver = Solver::new(engine);

    user.add_task_id(record.id);

    let mut output = Paginator::new(4096);
    let solution = solver.solve(task, Default::default(), EXECUTION_DEADLINE_DEFAULT);
    match solution.status {
        SolutionStatus::Answer(_) => {
            output
                .write_str("Solution: ")
                .map_err(|e| format!("error {e}"))?;
        }
        SolutionStatus::Err(e) => {
            output
                .write_str(&format!("Solution: {e} "))
                .map_err(|e| format!("error {e}"))?;
        }
        _ => unreachable!(),
    };

    View::try_from(solution.as_ref())
        .unwrap()
        .display_impl(&mut Html {
            output: &mut output,
        })
        .unwrap();

    // TODO: answer validation

    record.runs.push(solution.cycles());
    tasks.put(&record).map_err(|e| e.to_string())?;

    Ok(output)
}

pub async fn handler(
    api: &Api,
    command: Command,
    engine: Arc<RulesEngine>,
    tasks: Arc<TaskDb>,
    users: Arc<UserDb>,
) {
    let text = format!("{} {}", command.command, command.args);
    let user_id = i64::from(command.user_id) as u64;
    let mut user = users
        .get(user_id)
        .unwrap_or_default()
        .unwrap_or_else(|| UserRecord::new(user_id));

    match task(text, engine.clone(), tasks.clone(), &mut user) {
        Ok(s) => {
            for i in s.iter() {
                println!("{i}");
                api.send(command.chat_id.text(i).parse_mode(ParseMode::Html))
                    .await
                    .unwrap();
            }
        }
        Err(s) => {
            api.send(command.chat_id.text(s).parse_mode(ParseMode::Html))
                .await
                .unwrap();
        }
    };

    users.put(&user).unwrap();
}
