use std::sync::Arc;

use rocket::response::status::Custom;

use super::super::{BoardId, BOARD_NOT_FOUND};
use bbs::Bbs;
use bbs::board::SubjectTxt;

#[get("/<board>/subject.txt")]
pub fn get<'r>(board: BoardId, bbs: &'r Bbs)
    -> Result<Arc<SubjectTxt>, Custom<&'static str>>
{
    let brd = bbs.board(&*board).ok_or(BOARD_NOT_FOUND)?;
    Ok(brd.subject_txt())
}
