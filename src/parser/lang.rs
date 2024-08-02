use std::convert::From;

use crate::{error::ParserError, grammar::ra, Location, NodeData, Tree};

pub fn symbol(text: &str) -> Result<Tree, ParserError> {
    let state = ra::symbol(text).map_err(ParserError::from)?;
    let poses = Location::new_line_poses(text);

    Ok(Tree::from(state.into_bfs().map(|x| NodeData {
        location: Location::from_position(x.position, x.symbol.len(), &poses),
        symbol:   x.symbol,
    })))
}

pub fn terms(text: &str) -> Result<Vec<Tree>, ParserError> {
    let state = ra::terms(text).map_err(ParserError::from)?;
    let poses = Location::new_line_poses(text);

    Ok(state
        .into_iter()
        .map(|s| {
            Tree::from(s.into_bfs().map(|x| NodeData {
                location: Location::from_position(x.position, x.symbol.len(), &poses),
                symbol:   x.symbol,
            }))
        })
        .collect())
}

pub fn lang_rule(text: &str) -> Result<Tree, ParserError> {
    let state = ra::lang_rule(text).map_err(ParserError::from)?;
    let poses = Location::new_line_poses(text);

    Ok(Tree::from(state.into_bfs().map(|x| NodeData {
        location: Location::from_position(x.position, x.symbol.len(), &poses),
        symbol:   x.symbol,
    })))
}

pub fn task(text: &str) -> Result<Tree, ParserError> {
    let state = ra::task(text).map_err(ParserError::from)?;
    let poses = Location::new_line_poses(text);

    Ok(Tree::from(state.into_bfs().map(|x| NodeData {
        location: Location::from_position(x.position, x.symbol.len(), &poses),
        symbol:   x.symbol,
    })))
}

pub fn any(text: &str) -> Result<Vec<Tree>, ParserError> {
    let state = ra::any(text).map_err(ParserError::from)?;
    let poses = Location::new_line_poses(text);

    Ok(state
        .into_iter()
        .map(|s| {
            Tree::from(s.into_bfs().map(|x| NodeData {
                location: Location::from_position(x.position, x.symbol.len(), &poses),
                symbol:   x.symbol,
            }))
        })
        .collect())
}
