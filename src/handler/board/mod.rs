pub mod dat;
pub mod setting_txt;
pub mod subject_txt;

use super::BoardId;

#[get("/<_board>")]
pub fn get(_board: BoardId) {
    unimplemented!();
}
