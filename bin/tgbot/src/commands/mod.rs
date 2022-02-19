mod problem;
mod problems_list;

use futures::{future::Future, Stream};
use telebot::{functions::*, Bot};

use crate::text::Text;

pub use problem::handler as problem_handler;
pub use problems_list::handler as problems_list_handler;

pub fn start_handler(bot: &mut Bot) -> impl Future<Item = (), Error = failure::Error> {
    bot.new_cmd("/start")
        .and_then(|(bot, msg)| {
            let text = Text::start();
            bot.message(msg.chat.id, text)
                .parse_mode(ParseMode::HTML)
                .send()
        })
        .for_each(|_| Ok(()))
}
