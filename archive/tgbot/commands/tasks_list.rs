use std::sync::Arc;

use rust_i18n::t;
use telegram_bot::*;

use database::{TaskDb, TaskRecord, UserDb, UserRecord};

use crate::text::Text;

use super::Command;

fn tasks_list(user: &UserRecord, tasks: Arc<TaskDb>) -> Result<Vec<TaskRecord>, String> {
    let mut result = vec![];

    for id in user.tasks_iter() {
        if let Some(p) = tasks.get(id).map_err(|e| format!("Db error {e}"))? {
            result.push(p);
        }
    }

    Ok(result)
}

pub async fn handler(api: &Api, command: Command, tasks: Arc<TaskDb>, users: Arc<UserDb>) {
    let user_id = i64::from(command.user_id) as u64;
    let user = users
        .get(user_id)
        .unwrap_or_default()
        .unwrap_or_else(|| UserRecord::new(user_id));
    match tasks_list(&user, tasks.clone()) {
        Ok(tasks) if tasks.is_empty() => {
            api.send(
                command
                    .chat_id
                    .text(t!("content.empty_tasks_list"))
                    .parse_mode(ParseMode::Html),
            )
            .await
            .unwrap();
        }
        Ok(tasks) => {
            for p in tasks {
                let mut markup = types::InlineKeyboardMarkup::new();
                markup.add_row(vec![
                    types::InlineKeyboardButton::callback(
                        t!("buttons.rerun"),
                        format!("/rerun {}", p.id),
                    ),
                    types::InlineKeyboardButton::callback(
                        t!("buttons.report"),
                        format!("/report {}", p.id),
                    ),
                ]);
                api.send(
                    command
                        .chat_id
                        .text(Text::task_text(&p))
                        .parse_mode(ParseMode::Html)
                        .reply_markup(types::ReplyMarkup::InlineKeyboardMarkup(markup)),
                )
                .await
                .unwrap();
            }
        }
        Err(s) => {
            api.send(command.chat_id.text(s).parse_mode(ParseMode::Html))
                .await
                .unwrap();
        }
    }
}
