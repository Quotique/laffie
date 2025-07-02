use std::{convert::TryFrom, fmt::Write, str::FromStr, sync::Arc};

use rust_i18n::t;
use telegram_bot::*;

use database::{TaskDb, UserDb, UserRecord};
use solver::{
    rule::RulesEngine,
    task::{DumperConfig, Solver, EXECUTION_DEADLINE_DEFAULT},
    CompactString,
};
use view::{Html, View};

use super::Command;
use crate::pagination::Paginator;

fn rerun(
    task_id: CompactString,
    engine: Arc<RulesEngine>,
    tasks: Arc<TaskDb>,
    _user: &mut UserRecord,
) -> Result<Paginator, String> {
    let task_id: u128 = u128::from_str(&task_id).map_err(|e| e.to_string())?;

    let mut record = tasks
        .get(task_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| t!("errors.task_not_found"))?;

    let mut solution = Solver::new(
        engine,
        DumperConfig {
            sink:     "none".into(),
            filename: None,
        }
        .build(),
        EXECUTION_DEADLINE_DEFAULT,
    );

    let result = solution.solve(record.clone().into());
    let mut output = Paginator::new(4096);
    let solution = match result {
        Ok(solution) => {
            output
                .write_str("Solution:")
                .map_err(|e| format!("error {e}"))?;
            solution
        }
        Err((solution, e)) => {
            output
                .write_str(&format!("Solution: {e}"))
                .map_err(|e| format!("error {e}"))?;
            solution
        }
    };

    View::try_from(&solution)
        .unwrap()
        .display_impl(&mut Html {
            output: &mut output,
        })
        .unwrap();

    // TODO: answer validation

    record.runs.push(solution.current_cycles());
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
    let user_id = i64::from(command.user_id) as u64;
    let mut user = users
        .get(user_id)
        .unwrap_or_default()
        .unwrap_or_else(|| UserRecord::new(user_id));

    match rerun(command.args, engine.clone(), tasks.clone(), &mut user) {
        Ok(s) => {
            for i in s.iter() {
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
