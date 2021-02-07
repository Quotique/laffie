use super::frame::Frame;
use crate::{
    parser::SemanticError,
    core::term::Term,
    statement::{MarkedStatement, Statement},
};
use std::{convert::TryFrom, sync::Arc};
use trees::tr;

pub enum Target {
    Find(MarkedStatement),
    Proof(Frame),
    Transform(MarkedStatement),
}

impl TryFrom<Statement> for Target {
    type Error = SemanticError;

    fn try_from(mut value: Statement) -> Result<Self, Self::Error> {
        let (root, mut childs) = value.destruct();

        if root.data.is_symbol_name(&"find".into()) {
            if childs.degree() != 1 {
                return Err(SemanticError::WorngArgCount(format!("")));
            }
            return Ok(Self::Find(MarkedStatement::from(Arc::new(
                Statement::from(childs.pop_front().unwrap()),
            ))));
        } else if root.data.is_symbol_name(&"proof".into()) {
            if childs.degree() != 1 {
                return Err(SemanticError::WorngArgCount(format!("")));
            }

            let mut frame = Frame::default();
            frame.add_condition(MarkedStatement::from(Arc::new(Statement::from(
                childs.pop_front().unwrap(),
            ))));
            return Ok(Self::Proof(frame));
        } else if root.data.is_symbol_name(&"transform".into()) {
            if childs.degree() != 1 {
                return Err(SemanticError::WorngArgCount(format!("")));
            }
            return Ok(Self::Transform(MarkedStatement::from(Arc::new(
                Statement::from(childs.pop_front().unwrap()),
            ))));
        } else {
            Err(SemanticError::UnexpectedWord(root.to_string()))
        }
    }
}

impl Target {

}
