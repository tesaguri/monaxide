mod cacheable;

pub use self::cacheable::{Cacheable, Metadata};

use std::fs::File;
use std::io::{self, Read};
use std::marker::PhantomData;

use rocket::http::Status;
use rocket::request::{FromRequest, Outcome, Request, State};
use rocket::response::{
    DEFAULT_CHUNK_SIZE,
    Body,
    Responder,
    Response,
    ResponseBuilder,
};

#[derive(Debug)]
pub struct Bytes<'a>(pub &'a [u8]);

/// The optional type parameter `T` is used to differentiate each state in a `Rocket`.
///
/// ```
/// enum File1 {}
/// enum File2 {}
///
/// let f1 = StaticFile::<File1>::new(file1)?;
/// let f2 = StaticFile::<File2>::new(file2)?;
///
/// let rocket = rocket::ignite().manage(f1).manage(f2);
/// ```
#[derive(Default)]
pub struct StaticFile<T=()> {
    inner: Cacheable<Box<[u8]>>,
    tag: PhantomData<fn() -> T>,
}

impl<'a> Responder<'a> for Bytes<'a> {
    fn respond_to(self, _: &Request) -> Result<Response<'a>, Status> {
        slice_body(&mut Response::build(), self.0.as_ref()).ok()
    }
}

impl<T> StaticFile<T> {
    pub fn new(mut f: &File) -> io::Result<Self> {
        let m = f.metadata()?;
        let mut buf = Vec::with_capacity(m.len() as usize + 1);
        f.read_to_end(&mut buf)?;

        Ok(StaticFile {
            inner: Cacheable::new(buf.into(), (&m).into()),
            tag: PhantomData,
        })
    }
}

impl<T> AsRef<[u8]> for StaticFile<T> {
    fn as_ref(&self) -> &[u8] {
        self.inner.as_ref()
    }
}

impl<'a, 'r, T: Send+Sync+'static> FromRequest<'a, 'r> for &'r StaticFile<T> {
    type Error = ();

    fn from_request(req: &'a Request<'r>) -> Outcome<Self, Self::Error> {
        req.guard::<State<StaticFile<T>>>().map(|state| state.inner())
    }
}

impl<'r, T> Responder<'r> for &'r StaticFile<T> {
    fn respond_to(self, req: &Request) -> Result<Response<'r>, Status> {
        self.inner.respond_to(req)
    }
}

fn slice_body<'a, 'r>(res: &'a mut ResponseBuilder<'r>, slice: &'r [u8])
    -> &'a mut ResponseBuilder<'r>
{
    cfg_if! {
        if #[cfg(any(
            target_pointer_width = "16",
            target_pointer_width = "32",
            target_pointer_width = "64",
        ))]
        {
            const fn u64_from_usize(n: usize) -> u64 {
                n as u64
            }
        } else {
            compile_error!("unsupported target_pointer_width");
        }
    }

    let len = u64_from_usize(slice.len());
    if len <= DEFAULT_CHUNK_SIZE {
        res.raw_body(Body::Sized(slice, len))
    } else {
        res.streamed_body(slice)
    }
}
