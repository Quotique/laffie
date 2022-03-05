use std::{str::FromStr, sync::Arc};

use rust_i18n::t;
use telegram_bot::*;

use database::{ProblemDb, UserDb, UserRecord};
use mcore::{
    problem::Solution,
    rule::RulesEngine,
    statement::term::CompactString,
    utils::{Dumper, DumperConfig},
};
use view::Html;

use super::Command;

fn rerun(
    problem_id: CompactString,
    engine: Arc<RulesEngine>,
    problems: Arc<ProblemDb>,
    _user: &mut UserRecord,
) -> Result<String, String> {
    let problem_id: u128 = u128::from_str(&problem_id).map_err(|e| e.to_string())?;

    let mut record = problems
        .get(problem_id)?
        .ok_or_else(|| t!("errors.problem_not_found"))?;

    let mut solution = Solution::new(
        record.clone().into(),
        engine,
        Dumper::new(DumperConfig {
            sink:     "none".into(),
            filename: "".to_owned(),
        }),
    );

    let result = match solution.solve() {
        Ok(_) => format!("{} {}", "Solution:", Html(&solution)),
        Err(e) => format!("{} {} {}", "Solution:", e, Html(&solution)),
    };
    record.runs.push(solution.perf_stats.clone());
    problems.put(&record)?;

    Ok(result)
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
        Ok(s) | Err(s) => api
            .send(command.chat_id.text(s).parse_mode(ParseMode::Html))
            .await
            .unwrap(),
    };

    users.put(&user).unwrap();
}
