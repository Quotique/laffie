use std::sync::Arc;

use futures::{future::Future, Stream};
use telebot::{functions::*, Bot};

use database::{ProblemDb, UserDb, UserRecord};

use crate::text::Text;

fn problems_list(user: &UserRecord, problems: Arc<ProblemDb>) -> Result<String, String> {
    let mut result = String::default();

    for id in user.problems_iter() {
        if let Some(p) = problems.get(id).map_err(|e| format!("Db error {}", e))? {
            result = format!("{}\n{}", result, Text::problem_text(&p));
        }
    }

    if result.is_empty() {
        result = Text::empty_problems_list();
    }

    Ok(result)
}

pub fn handler(
    bot: &mut Bot,
    problems: Arc<ProblemDb>,
    users: Arc<UserDb>,
) -> impl Future<Item = (), Error = failure::Error> {
    bot.new_cmd("/problems_list")
        .and_then(move |(bot, msg)| {
            let user_id = msg.from.unwrap().id as u64;
            let user = users
                .get(user_id)
                .unwrap_or_default()
                .unwrap_or_else(|| UserRecord::new(user_id));
            match problems_list(&user, problems.clone()) {
                Ok(s) | Err(s) => bot
                    .message(msg.chat.id, s)
                    .parse_mode(ParseMode::HTML)
                    .send(),
            }
        })
        .for_each(|_| Ok(()))
}
