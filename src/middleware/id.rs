use std::io::{self, Write};
use std::net::SocketAddr;

use typemap::{Key, ShareMap};

use super::{AfterMiddleware, BeforeMiddleware, Request, Result};
use super::cap::Cap;
use post::Post;
use setting::{self, Settings};

pub struct Id;

impl Key for Id {
    type Value = IdHash;
}

pub struct IdHash {
    hash: u64,
    suffix: u8,
}

impl BeforeMiddleware for Id {
    fn before<'a, 'r, 'b, 'k>(
        &self, data: &mut ShareMap, _: &Post, req: &Request<'a, 'r, 'b, 'k>, settings: &Settings
    ) -> Result<'r, ()>
    {
        fn generate_id(_addr: SocketAddr) -> IdHash {
            IdHash {
                hash: 0,
                suffix: b'0',
            };
            unimplemented!();
        }

        if (! data.contains::<Cap>()) && settings.get::<setting::common::ForceId>().cloned().unwrap_or(true) {
            if let Some(r) = req.remote() {
                data.insert::<Id>(generate_id(r));
            } else {
                return Err((b"Remote address unknown" as &[u8]).into());
            }
        }

        Ok(())
    }
}

impl AfterMiddleware for Id {
    fn after(&self, post: &mut Post, data: &ShareMap, settings: &Settings) -> Result<'static, ()> {
        let dt = post.datetime_mut();
        if let Some(_id) = data.get::<Id>() {
            // "ID:abcdefgh0"
            super::reserve_and_delimit(dt, 12);
            dt.extend_from_slice(b"ID:");
            unimplemented!();
        } else if settings.get::<setting::common::ForceId>().cloned().unwrap_or(true) {
            super::reserve_and_delimit(dt, 6);
            dt.extend_from_slice(b"ID:???");
        }

        Ok(())
    }
}

impl IdHash {
    fn write_to<W: Write>(_w: W) -> io::Result<()> {
        unimplemented!();
    }
}
