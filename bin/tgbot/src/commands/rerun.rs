use std::{convert::TryFrom, fmt::Write, str::FromStr, sync::Arc};

use rust_i18n::t;
use telegram_bot::*;

use database::{ProblemDb, UserDb, UserRecord};
use mcore::{
    problem::Solution,
    rule::RulesEngine,
    statement::term::CompactString,
    utils::{Dumper, DumperConfig},
};
use view::{Html, View};

use super::Command;
use crate::pagination::Paginator;

fn rerun(
    problem_id: CompactString,
    engine: Arc<RulesEngine>,
    problems: Arc<ProblemDb>,
    _user: &mut UserRecord,
) -> Result<Paginator, String> {
    let problem_id: u128 = u128::from_str(&problem_id).map_err(|e| e.to_string())?;

    let mut record = problems
        .get(problem_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| t!("errors.problem_not_found"))?;

    let mut solution = Solution::new(
        record.clone().into(),
        engine,
        Dumper::new(DumperConfig {
            sink:     "none".into(),
            filename: "".to_owned(),
        }),
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

    record.runs.push(solution.perf_stats.clone());
    problems.put(&record).map_err(|e| e.to_string())?;

    Ok(output)
}

pub async fn handler(
    api: &Api,
    command: Command,
    engine: Arc<RulesEngine>,
    problems: Arc<ProblemDb>,
    users: Arc<UserDb>,
) {
    let user_id = i64::from(command.user_id) as u64;
    let mut user = users
        .get(user_id)
        .unwrap_or_default()
        .unwrap_or_else(|| UserRecord::new(user_id));

    match rerun(command.args, engine.clone(), problems.clone(), &mut user) {
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
