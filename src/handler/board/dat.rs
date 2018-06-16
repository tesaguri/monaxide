use rocket::http::RawStr;
use rocket::request::FromParam;

use super::super::BoardId;

#[get("/<_board>/dat/<_dat>")]
pub fn get(_board: BoardId, _dat: Dat) {
    unimplemented!();
}

pub struct Dat(usize);

impl<'a> FromParam<'a> for Dat {
    type Error = ();

    fn from_param(p: &'a RawStr) -> Result<Self, ()> {
        if p.ends_with(".dat") {
            p[..(p.len()-4)].parse().map(Dat).map_err(|_| ())
        } else {
            Err(())
        }
    }
}
