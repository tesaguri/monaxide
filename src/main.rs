#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate monaxide;
extern crate rocket;

use monaxide::middleware::datetime::DateTime;

fn main() {
    use monaxide::handler::*;

    let mut bbs = monaxide::Bbs::new().unwrap();
    bbs.attach(DateTime::with_jst());

    rocket::ignite()
        .manage(bbs)
        .mount("/", routes![board::get, board::dat::get, board::setting_txt::get])
        .mount("/test", routes![test::bbs::post, test::read::get])
        .launch();
}
