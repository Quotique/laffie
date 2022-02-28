mod problem;
mod problems_list;

use std::sync::Arc;

use rust_i18n::t;
use telegram_bot::*;

use database::{ProblemDb, UserDb};
use mcore::{rule::RulesEngine, statement::term::CompactString};

use crate::text::Text;

const MISSING_COMMAND: &str = "missing command";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Command {
    chat_id: types::ChatId,
    user_id: types::UserId,
    command: CompactString,
    args:    CompactString,
}

pub async fn process_update(
    update: types::Update,
    api: &Api,
    engine: Arc<RulesEngine>,
    problems: Arc<ProblemDb>,
    users: Arc<UserDb>,
) {
    rust_i18n::set_locale("ru");
    match update.kind {
        UpdateKind::CallbackQuery(query) => {
            let _ = api.send(query.acknowledge()).await;
            if let (Some(data), Some(message)) = (&query.data, &query.message) {
                let chat_id = message.to_source_chat();
                match Command::new(query.from.id, chat_id, data) {
                    Ok(command) => command.handle(api, engine, problems, users).await,
                    Err(e) => error_handler(api, chat_id, &e.to_string()).await,
                }
            }
        }
        UpdateKind::Message(message) => {
            if let MessageKind::Text { ref data, .. } = &message.kind {
                match Command::new(message.from.id, message.chat.id(), data) {
                    Ok(command) => command.handle(api, engine, problems, users).await,
                    Err(e) => error_handler(api, message.chat.id(), &e.to_string()).await,
                }
            }
        }
        _ => {
            println!("{:?}", update);
        }
    }
}

async fn start_handler(api: &Api, command: Command) {
    let mut markup = types::InlineKeyboardMarkup::new();
    markup.add_row(vec![
        types::InlineKeyboardButton::callback(t!("buttons.help"), "/help"),
        types::InlineKeyboardButton::callback(t!("buttons.problems"), "/problems_list"),
    ]);
    markup.add_row(vec![
        types::InlineKeyboardButton::callback(t!("buttons.guide"), "/guide"),
        types::InlineKeyboardButton::callback(t!("buttons.examples"), "/examples"),
    ]);
    api.send(
        command
            .chat_id
            .text(t!(
                "content.start",
                version = &Text::version(),
                system = &Text::system()
            ))
            .parse_mode(ParseMode::Html)
            .reply_markup(types::ReplyMarkup::InlineKeyboardMarkup(markup)),
    )
    .await
    .unwrap();
}

async fn error_handler(api: &Api, chat_id: ChatId, error: &str) {
    api.send(chat_id.text(error).parse_mode(ParseMode::Html))
        .await
        .unwrap();
}

impl Command {
    pub fn new(
        user_id: types::UserId,
        chat_id: types::ChatId,
        value: &str,
    ) -> Result<Self, &'static str> {
        let mut iter = value.splitn(2, '/');
        let _ = iter.next();
        let command = iter.next().ok_or(MISSING_COMMAND)?;
        let mut iter = command.splitn(2, ' ');
        let command = iter.next().ok_or(MISSING_COMMAND)?;
        let args = iter.next().unwrap_or_else(|| &value[0..0]);

        Ok(Self {
            user_id,
            chat_id,
            command: command.into(),
            args: args.into(),
        })
    }

    async fn handle(
        self,
        api: &Api,
        engine: Arc<RulesEngine>,
        problems: Arc<ProblemDb>,
        users: Arc<UserDb>,
    ) {
        match self.command.as_str() {
            "start" => start_handler(api, self).await,
            "problem" => problem::handler(api, self, engine, problems, users).await,
            "problems_list" => problems_list::handler(api, self, problems, users).await,
            "help" => {
                api.send(
                    self.chat_id
                        .text(t!("content.help"))
                        .parse_mode(ParseMode::Html),
                )
                .await
                .unwrap();
            }
            "guide" => {
                api.send(
                    self.chat_id
                        .text(t!("content.instruction"))
                        .parse_mode(ParseMode::Html),
                )
                .await
                .unwrap();
            }
            "examples" => {
                api.send(
                    self.chat_id
                        .text(t!("content.examples"))
                        .parse_mode(ParseMode::Html),
                )
                .await
                .unwrap();
            }
            _ => error_handler(api, self.chat_id, &t!("errors.unknown_command")).await,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_parse_test() {
        let user_id = types::UserId::from(100);
        let chat_id = types::ChatId::from(100);
        for (test, result) in &[
            (
                "/start",
                Command {
                    user_id,
                    chat_id,
                    command: "start".into(),
                    args: "".into(),
                },
            ),
            (
                "/problem asdf",
                Command {
                    user_id,
                    chat_id,
                    command: "problem".into(),
                    args: "asdf".into(),
                },
            ),
            (
                "prefix /problem asdf qwer",
                Command {
                    user_id,
                    chat_id,
                    command: "problem".into(),
                    args: "asdf qwer".into(),
                },
            ),
        ] {
            assert_eq!(&Command::new(user_id, chat_id, test).unwrap(), result);
        }
    }
}
