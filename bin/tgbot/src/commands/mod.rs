mod problem;
mod problems_list;

use std::{str::FromStr, sync::Arc};

use telegram_bot::*;

use database::{ProblemDb, UserDb};
use mcore::{rule::RulesEngine, statement::term::CompactString};

use crate::text::Text;

const MISSING_COMMAND: &str = "missing command";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Command {
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
    if let UpdateKind::Message(message) = update.kind {
        if let MessageKind::Text { ref data, .. } = &message.kind {
            match Command::from_str(data) {
                Ok(command) => match command.command.as_str() {
                    "start" => start_handler(api, &message).await,
                    "problem" => {
                        problem::handler(api, &message, command, engine, problems, users).await
                    }
                    "problems_list" => problems_list::handler(api, &message, problems, users).await,
                    _ => error_handler(api, &message, "unknown command").await,
                },
                Err(e) => error_handler(api, &message, &e.to_string()).await,
            }
        }
    }
}

async fn start_handler(api: &Api, message: &Message) {
    let text = Text::start();
    let chat = message.chat.clone();
    api.send(chat.text(text).parse_mode(ParseMode::Html))
        .await
        .unwrap();
}

async fn error_handler(api: &Api, message: &Message, error: &str) {
    let chat = message.chat.clone();
    api.send(chat.text(error).parse_mode(ParseMode::Html))
        .await
        .unwrap();
}

impl FromStr for Command {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let mut iter = value.splitn(2, '/');
        let _ = iter.next();
        let command = iter.next().ok_or(MISSING_COMMAND)?;
        let mut iter = command.splitn(2, ' ');
        let command = iter.next().ok_or(MISSING_COMMAND)?;
        let args = iter.next().unwrap_or_else(|| &value[0..0]);

        Ok(Self {
            command: command.into(),
            args:    args.into(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_parse_test() {
        for (test, result) in &[
            (
                "/start",
                Command {
                    command: "start".into(),
                    args:    "".into(),
                },
            ),
            (
                "/problem asdf",
                Command {
                    command: "problem".into(),
                    args:    "asdf".into(),
                },
            ),
            (
                "prefix /problem asdf qwer",
                Command {
                    command: "problem".into(),
                    args:    "asdf qwer".into(),
                },
            ),
        ] {
            assert_eq!(&Command::from_str(test).unwrap(), result);
        }
    }
}
