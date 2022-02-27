use std::sync::Arc;

use telegram_bot::*;

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

pub async fn handler(api: &Api, message: &Message, problems: Arc<ProblemDb>, users: Arc<UserDb>) {
    let user_id = i64::from(message.from.id) as u64;
    let user = users
        .get(user_id)
        .unwrap_or_default()
        .unwrap_or_else(|| UserRecord::new(user_id));
    let chat = message.chat.clone();
    match problems_list(&user, problems.clone()) {
        Ok(s) | Err(s) => {
            api.send(chat.text(s).parse_mode(ParseMode::Html))
                .await
                .unwrap();
        }
    }
}
