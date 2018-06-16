use rocket::response::status::Custom;

use super::super::{BoardId, BOARD_NOT_FOUND};
use bbs::Bbs;
use setting::Settings;

#[get("/<board>/SETTING.TXT")]
pub fn get<'r>(board: BoardId, bbs: &'r Bbs)
    -> Result<&'r Settings, Custom<&'static str>>
{
    let brd = bbs.board(&*board).ok_or(BOARD_NOT_FOUND)?;
    Ok(brd.settings())
}
