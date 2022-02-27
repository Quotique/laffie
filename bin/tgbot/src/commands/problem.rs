use std::sync::Arc;

use telegram_bot::*;

use database::{ProblemDb, ProblemRecord, UserDb, UserRecord};
use mcore::{
    problem::Solution,
    rule::RulesEngine,
    utils::{Dumper, DumperConfig},
};
use parser::{lang, ProblemParser};

use super::Command;

fn problem(
    problem_text: String,
    engine: Arc<RulesEngine>,
    problems: Arc<ProblemDb>,
    user: &mut UserRecord,
) -> Result<String, String> {
    let states = lang::problem(&problem_text)
        .map_err(|e| format!("<code>{}</code>", e.error_string(&problem_text)))?;
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

pub async fn handler(
    api: &Api,
    message: &Message,
    command: Command,
    engine: Arc<RulesEngine>,
    problems: Arc<ProblemDb>,
    users: Arc<UserDb>,
) {
    let text = format!("{} {}", command.command, command.args);
    let user_id = i64::from(message.from.id) as u64;
    let mut user = users
        .get(user_id)
        .unwrap_or_default()
        .unwrap_or_else(|| UserRecord::new(user_id));

    let chat = message.chat.clone();

    match problem(text, engine.clone(), problems.clone(), &mut user) {
        Ok(s) | Err(s) => api
            .send(chat.text(s).parse_mode(ParseMode::Html))
            .await
            .unwrap(),
    };

    users.put(&user).unwrap();
}
