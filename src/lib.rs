#![feature(
    const_fn,
    custom_derive,
    label_break_value,
    never_type,
    plugin,
    read_initializer,
    try_from,
)]
#![plugin(rocket_codegen)]

extern crate ascii;
extern crate atoi;
#[macro_use]
extern crate cfg_if;
extern crate checked;
extern crate chrono;
extern crate hyper;
extern crate lazy_init;
extern crate memchr;
extern crate owning_ref;
extern crate parking_lot;
extern crate percent_encoding;
extern crate rocket;
extern crate time;
extern crate typemap;

pub mod bbs;
pub mod handler;
pub mod middleware;
pub mod post;
pub mod setting;

mod responder;
mod util;
mod validator;

pub use bbs::Bbs;
