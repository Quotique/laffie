use std::sync::Arc;

use futures::{future::Future, Stream};
use telebot::{functions::*, Bot};

use database::{ProblemDb, ProblemRecord, UserDb, UserRecord};
use mcore::{
    problem::Solution,
    rule::RulesEngine,
    utils::{Dumper, DumperConfig},
};
use parser::{ra, ProblemParser};

fn problem(
    problem_text: String,
    engine: Arc<RulesEngine>,
    problems: Arc<ProblemDb>,
    user: &mut UserRecord,
) -> Result<String, String> {
    let states = ra::problem(&problem_text).map_err(|e| e.to_string())?;
    let problem = ProblemParser::with(&states)
        .parse()
        .map_err(|e| e.to_string())?;

    let record = ProblemRecord::from(&problem);
    let record = problems.get_or_insert(record)?;

    let mut solution = Solution::new(
        problem,
        engine,
        Dumper::new(DumperConfig {
            sink:     "none".into(),
            filename: "".to_owned(),
        }),
    );

    user.add_problem_id(record.id);

    let result = match solution.solve() {
        Ok(_) => format!("{} {}", "Solution:", solution),
        Err(e) => format!("{} {} {}", "Solution:", e, solution),
    };
    let plain_bytes = strip_ansi_escapes::strip(result.as_bytes()).unwrap();
    Ok(std::str::from_utf8(&plain_bytes).unwrap().to_owned())
}

pub fn handler(
    bot: &mut Bot,
    engine: Arc<RulesEngine>,
    problems: Arc<ProblemDb>,
    users: Arc<UserDb>,
) -> impl Future<Item = (), Error = failure::Error> {
    bot.new_cmd("/problem")
        .and_then(move |(bot, msg)| {
            let text = format!("problem {}", msg.text.unwrap());
            let user_id = msg.from.unwrap().id as u64;
            let mut user = users
                .get(user_id)
                .unwrap_or_default()
                .unwrap_or_else(|| UserRecord::new(user_id));

            let result = match problem(text, engine.clone(), problems.clone(), &mut user) {
                Ok(s) | Err(s) => bot
                    .message(msg.chat.id, s)
                    .parse_mode(ParseMode::HTML)
                    .send(),
            };

            users.put(&user).unwrap();
            result
        })
        .for_each(|_| Ok(()))
}
