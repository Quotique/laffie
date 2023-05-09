use std::{str::FromStr, sync::Arc};

use rust_i18n::t;
use telegram_bot::*;

use database::{ProblemDb, UserDb, UserRecord};
use mcore::statement::term::CompactString;

use super::Command;

fn report(
    problem_id: CompactString,
    problems: Arc<ProblemDb>,
    user: &mut UserRecord,
) -> Result<String, String> {
    let problem_id: u128 = u128::from_str(&problem_id).map_err(|e| e.to_string())?;

    let mut record = problems
        .get(problem_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| t!("errors.problem_not_found"))?;

    if !record.reports.iter().any(|x| x == &user.id) {
        record.reports.push(user.id);
    }

    problems.put(&record).map_err(|e| e.to_string())?;

    Ok(t!("content.reported"))
}

pub async fn handler(api: &Api, command: Command, problems: Arc<ProblemDb>, users: Arc<UserDb>) {
    let user_id = i64::from(command.user_id) as u64;
    let mut user = users
        .get(user_id)
        .unwrap_or_default()
        .unwrap_or_else(|| UserRecord::new(user_id));

    match report(command.args, problems.clone(), &mut user) {
        Ok(s) | Err(s) => api
            .send(command.chat_id.text(s).parse_mode(ParseMode::Html))
            .await
            .unwrap(),
    };

    users.put(&user).unwrap();
}
