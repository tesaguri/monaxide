pub mod common;

use std::any::Any;
use std::collections::HashMap;
use std::default::Default;
use std::fs::File;
use std::io;

use lazy_init::LazyTransform;
use memchr::memchr;
use rocket::http::Status;
use rocket::request::Request;
use rocket::response::{Responder, Response};

use responder::StaticFile;
use util::erase_lifetime;

#[derive(Default)]
pub struct Settings {
    text: StaticFile,
    map: HashMap<&'static [u8], Item<'static>>,
}

pub trait Setting {
    const KEY: &'static str;
    type Value: Send+Sync+'static;

    fn from_raw(raw: &[u8]) -> Option<Self::Value>;
}

struct Item<'a> {
    typed: LazyTransform<&'a [u8], Option<Box<Any+Send+Sync>>>,
}

impl Settings {
    pub fn empty() -> Self {
        Default::default()
    }

    pub fn load(text: &File) -> io::Result<Self> {
        let text = StaticFile::new(&text)?;
        let mut map = HashMap::new();
        {
            let mut slice: &[u8] = text.as_ref();
            while ! slice.is_empty() {
                let line = if let Some(i) = memchr(b'\n', slice) {
                    unsafe {
                        let tmp = slice.get_unchecked(..i);
                        slice = slice.get_unchecked((i+1)..);
                        tmp
                    }
                } else {
                    let tmp = slice;
                    slice = &slice[slice.len()..];
                    tmp
                };
                if let Some(i) = memchr(b'=', line) {
                    unsafe {
                        let key = erase_lifetime(line.get_unchecked(..i));
                        let val = erase_lifetime(line.get_unchecked((i+1)..));
                        map.insert(key, Item::new(val));
                    }
                }
            }
        }

        map.shrink_to_fit();
        return Ok(Settings { text, map })
    }

    #[inline]
    pub fn get<S: Setting>(&self) -> Option<&S::Value> {
        self.map.get(S::KEY.as_bytes()).and_then(Item::typed::<S>)
    }
}

impl AsRef<[u8]> for Settings {
    fn as_ref(&self) -> &[u8] {
        self.text.as_ref()
    }
}

impl<'r> Responder<'r> for &'r Settings {
    fn respond_to(self, req: &Request) -> Result<Response<'r>, Status> {
        self.text.respond_to(req)
    }
}

impl<'a> Item<'a> {
    fn new(raw: &'a [u8]) -> Self {
        Item {
            typed: LazyTransform::new(raw),
        }
    }

    fn typed<S: Setting>(&self) -> Option<&S::Value> {
        self.typed
            .get_or_create(|raw| S::from_raw(raw).map(|t| Box::new(t) as _))
            .as_ref()
            .and_then(|b| Any::downcast_ref(b))
    }
}

#[test]
mod test {
    // TODO
}
