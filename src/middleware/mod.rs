pub mod cap;
pub mod datetime;
pub mod id;

mod middlewares;

pub use self::middlewares::Middlewares;

use std::borrow::Cow;
use std::net::SocketAddr;
use std::result;

use rocket::{self, State};
use rocket::http::Cookies;
use typemap::ShareMap;

use post::Post;
use setting::Settings;
use validator::{Digits, AlphaNum};

pub struct Request<'a, 'r: 'a+'b+'k, 'b, 'k> {
    board: AlphaNum<'b>,
    key: Digits<'k>,
    /// The underlying `rocket::Request`, necessary to acquire `State`s.
    rocket: &'a rocket::Request<'r>,
}

// pub struct Request<'a, 'r> {
//     board: &'r str,
//     key: u64,
//     key_str: &'r str,
//     number: u16,
//     name: Cow<'r, [u8]>,
//     mail: Cow<'r, [u8]>,
//     datetime: Vec<u8>,
//     body: Cow<'r, [u8]>,
//     rocket: &'a rocket::Request<'r>,
// }

// TODO: board id, thread key and post#
pub trait BeforeMiddleware {
    fn before<'a, 'r, 'b, 'k>(
        &self, data: &mut ShareMap, post: &Post, req: &Request<'a, 'r, 'b, 'k>, settings: &Settings
    ) -> Result<'r, ()>;
}

pub trait AfterMiddleware {
    fn after(&self, post: &mut Post, data: &ShareMap, settings: &Settings) -> Result<'static, ()>;
}

pub type Result<'a, T> = result::Result<T, Cow<'a, [u8]>>;

impl<'a, 'r, 'b, 'k> Request<'a, 'r, 'b, 'k> {
    pub fn new(board: AlphaNum<'b>, key: Digits<'k>, rocket: &'a rocket::Request<'r>) -> Self {
        Request {
            board,
            key,
            rocket,
        }
    }

    pub fn board(&self) -> &'b str {
        self.board.as_str()
    }

    pub fn cookies(&self) -> Cookies {
        self.rocket.cookies()
    }

    pub fn key(&self) -> u64 {
        self.key.number
    }

    pub fn key_str(&self) -> &'k str {
        self.key.as_str()
    }

    pub fn remote(&self) -> Option<SocketAddr> {
        self.rocket.remote()
    }

    pub fn state<T: Send+Sync+'static>(&self) -> Option<&'r T> {
        self.rocket.guard::<State<T>>().succeeded().map(|s| s.inner())
    }

    pub fn user_agent(&self) -> Option<&'a str> {
        self.rocket.headers().get_one("User-Agent")
    }
}

impl<F> BeforeMiddleware for F
    where F: for<'a, 'r, 'b, 'k> Fn(&mut ShareMap, &Post, &Request<'a, 'r, 'b, 'k>, &Settings) -> Result<'r, ()>
{
    fn before<'a, 'r, 'b, 'k>(
        &self, data: &mut ShareMap, post: &Post, req: &Request<'a, 'r, 'b, 'k>, settings: &Settings
    ) -> Result<'r, ()>
    {
        self(data, post, req, settings)
    }
}

impl<F> AfterMiddleware for F where F: Fn(&mut Post, &ShareMap, &Settings) -> Result<'static, ()> {
    fn after(&self, post: &mut Post, data: &ShareMap, settings: &Settings) -> Result<'static, ()> {
        self(post, data, settings)
    }
}

fn reserve_and_delimit(v: &mut Vec<u8>, additional: usize) {
    match v.last() {
        Some(&b' ') | None => v.reserve(additional),
        Some(_) => {
            v.reserve(additional + 1);
            v.push(b' ');
        },
    }
}
