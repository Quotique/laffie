use std::{str::FromStr, sync::Arc};

use rust_i18n::t;
use telegram_bot::*;

use database::{TaskDb, UserDb, UserRecord};
use mcore::CompactString;

use super::Command;

fn report(
    task_id: CompactString,
    tasks: Arc<TaskDb>,
    user: &mut UserRecord,
) -> Result<String, String> {
    let task_id: u128 = u128::from_str(&task_id).map_err(|e| e.to_string())?;

    let mut record = tasks
        .get(task_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| t!("errors.task_not_found"))?;

    if !record.reports.iter().any(|x| x == &user.id) {
        record.reports.push(user.id);
    }

    tasks.put(&record).map_err(|e| e.to_string())?;

    Ok(t!("content.reported"))
}

pub async fn handler(api: &Api, command: Command, tasks: Arc<TaskDb>, users: Arc<UserDb>) {
    let user_id = i64::from(command.user_id) as u64;
    let mut user = users
        .get(user_id)
        .unwrap_or_default()
        .unwrap_or_else(|| UserRecord::new(user_id));

    match report(command.args, tasks.clone(), &mut user) {
        Ok(s) | Err(s) => api
            .send(command.chat_id.text(s).parse_mode(ParseMode::Html))
            .await
            .unwrap(),
    };

    users.put(&user).unwrap();
}
