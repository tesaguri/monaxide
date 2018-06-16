use rocket::http::Status;
use rocket::response::status::Custom;

use validator;

pub mod board;
pub mod test;

pub type BoardId<'a> = validator::AlphaNum<'a>;
pub type Key<'a> = validator::Digits<'a>;

const BOARD_NOT_FOUND: Custom<&str> = Custom(
    Status::NotFound,
    "Board not found",
);
