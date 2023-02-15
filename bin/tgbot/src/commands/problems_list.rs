use std::sync::Arc;

use rust_i18n::t;
use telegram_bot::*;

use database::{ProblemDb, ProblemRecord, UserDb, UserRecord};

use crate::text::Text;

use super::Command;

fn problems_list(
    user: &UserRecord,
    problems: Arc<ProblemDb>,
) -> Result<Vec<ProblemRecord>, String> {
    let mut result = vec![];

    for id in user.problems_iter() {
        if let Some(p) = problems.get(id).map_err(|e| format!("Db error {e}"))? {
            result.push(p);
        }
    }

    Ok(result)
}

pub async fn handler(api: &Api, command: Command, problems: Arc<ProblemDb>, users: Arc<UserDb>) {
    let user_id = i64::from(command.user_id) as u64;
    let user = users
        .get(user_id)
        .unwrap_or_default()
        .unwrap_or_else(|| UserRecord::new(user_id));
    match problems_list(&user, problems.clone()) {
        Ok(problems) if problems.is_empty() => {
            api.send(
                command
                    .chat_id
                    .text(t!("content.empty_problems_list"))
                    .parse_mode(ParseMode::Html),
            )
            .await
            .unwrap();
        }
        Ok(problems) => {
            for p in problems {
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
                        .text(Text::problem_text(&p))
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
