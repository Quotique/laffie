use std::{convert::TryFrom, fmt::Write, str::FromStr, sync::Arc};

use rust_i18n::t;
use telegram_bot::*;

use database::{TaskDb, UserDb, UserRecord};
use mcore::{
    rule::RulesEngine,
    task::{Dumper, DumperConfig, Solution},
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

    let mut solution = Solution::new(
        record.clone().into(),
        engine,
        Dumper::new(DumperConfig {
            sink:     "none".into(),
            filename: "".to_owned(),
        }),
        Default::default(),
    );

    let mut output = Paginator::new(4096);
    output
        .write_str(&match solution.solve() {
            Ok(_) => format!("{} ", "Solution:"),
            Err(e) => format!("{} {} ", "Solution:", e,),
        })
        .map_err(|e| format!("error {e}"))?;

    View::try_from(&solution)
        .unwrap()
        .display_impl(&mut Html {
            output: &mut output,
        })
        .unwrap();

    if let Some(answer) = solution.answer() {
        // TODO: answer changed
        record.answer = answer.as_ref().clone();
    }
    record.runs.push(solution.cycles);
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
