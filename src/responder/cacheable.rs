use std::fs;
use std::ops::Deref;
use std::str::{self, FromStr};

use rocket::http::{Header, Status};
use rocket::request::Request;
use rocket::response::{Responder, Response};
use time::{self, Timespec};

#[derive(Clone, Default)]
pub struct Cacheable<T> {
    body: T,
    metadata: Metadata,
}

#[derive(Clone)]
pub struct Metadata {
    etag: Box<str>,
    modified: Box<str>,
    mtime: Timespec,
}

/// Yet another implementation of `hyper::header::ByteRangeSpec`
/// because that only supports `u64` range whereas we need `usize`.
struct ByteRangesSpecifier<T>(T, Option<T>);

impl<T> Cacheable<T> {
    pub fn new(body: T, metadata: Metadata) -> Self {
        Cacheable { body, metadata }
    }

    pub fn modify(&mut self, id: u64) -> &mut T {
        self.metadata.modify(id);
        &mut self.body
    }

    pub fn body(&self) -> &T {
        &self.body
    }

    pub fn body_mut(&mut self) -> &mut T {
        &mut self.body
    }
}

impl<T> Deref for Cacheable<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.body
    }
}

impl<'r, T> Responder<'r> for &'r Cacheable<T> where T: AsRef<[u8]> {
    fn respond_to(self, req: &Request) -> Result<Response<'r>, Status> {
        respond_to(req, self.body.as_ref(), &self.metadata)
    }
}

const ETAG_INNER_LEN: usize = 11;
const ETAG_LEN: usize = ETAG_INNER_LEN + 2;
const RFC822_LEN: usize = 29;

impl Metadata {
    pub fn now(id: u64) -> Self {
        Metadata::new(id, time::get_time())
    }

    pub fn modify(&mut self, id: u64) {
        self.set_mtime(time::get_time());
        self.set_etag(id);
    }

    fn new(id: u64, mtime: Timespec) -> Self {
        unsafe fn alloc_boxed_str(cap: usize) -> Box<str> {
            let mut buf = String::with_capacity(cap);
            buf.as_mut_vec().set_len(cap);
            buf.into()
        }

        unsafe {
            let mut etag = alloc_boxed_str(ETAG_LEN);
            {
                let bytes = etag.as_bytes_mut();
                assert_eq!(bytes.len(), ETAG_LEN);
                bytes[0] = b'"';
                bytes[ETAG_LEN-1] = b'"';
            }
            let mut ret = Metadata {
                etag,
                modified: alloc_boxed_str(RFC822_LEN),
                mtime,
            };
            ret.set_etag(id);
            debug_assert!(str::from_utf8(ret.etag.as_bytes()).is_ok());
            ret.set_mtime(mtime);
            debug_assert!(str::from_utf8(ret.modified.as_bytes()).is_ok());
            ret
        }
    }

    fn set_etag(&mut self, id: u64) {
        const B64_ENC: &[u8; 64] =
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

        let mut hash = id.overflowing_mul(self.mtime.sec as u64).0;
        unsafe {
            for b in &mut self.etag.as_bytes_mut()[1..ETAG_INNER_LEN+1] {
                *b = B64_ENC[(hash | 0b111111) as usize];
                hash >>= 6;
            }
        }
    }

    fn set_mtime(&mut self, mtime: Timespec) {
        self.mtime = mtime;
        self.set_modified();
    }

    fn set_modified(&mut self) {
        use std::io::Write;

        unsafe {
            let mut slice: &mut [u8] = self.modified.as_bytes_mut();
            write!(slice, "{}", time::at_utc(self.mtime).rfc822()).unwrap();
            assert!(slice.is_empty());
            debug_assert!(str::from_utf8(slice).is_ok());
        }
    }
}

impl Default for Metadata {
    fn default() -> Self {
        Metadata::new(0, Timespec::new(i64::min_value(), 0))
    }
}

impl<'a> From<&'a fs::Metadata> for Metadata {
    fn from(m: &'a fs::Metadata) -> Self {
        Metadata::new(metadata::id(m), metadata::mtime(m))
    }
}

impl<T: Copy+FromStr+PartialOrd> FromStr for ByteRangesSpecifier<T> {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, ()> {
        // https://tools.ietf.org/html/rfc7233#section-2.1
        let mut parts = s.splitn(2, '-');
        match (parts.next().map(T::from_str), parts.next()) {
            (Some(Ok(s)), Some("")) => Ok(ByteRangesSpecifier(s, None)),
            (Some(Ok(s)), Some(end)) => match end.parse() {
                Ok(e) if s <= e => Ok(ByteRangesSpecifier(s, Some(e))),
                _ => Err(()),
            },
            _ => Err(()),
        }
    }
}

fn respond_to<'r>(req: &Request, body: &'r [u8], metadata: &'r Metadata)
    -> Result<Response<'r>, Status>
{
    use rocket::http::hyper::header::*;

    macro_rules! h {
        ($name:ident) => {
            <::hyper::header::$name as ::hyper::header::Header>::header_name()
        };
    }

    let headers = req.headers();
    let mut res = Response::build();
    res.header(Header::new(h!(AcceptRanges), "bytes"));

    if headers.get(h!(IfNoneMatch)).any(|v| *v == *metadata.etag) {
        return res.status(Status::NotModified).ok();
    }

    if let Some(ims) = headers.get_one(h!(IfModifiedSince)) {
        if let Ok(HttpDate(tm)) = ims.parse() {
            if tm.to_timespec().sec < metadata.mtime.sec {
                return res.status(Status::NotModified).ok();
            }
        }
    }

    if headers.contains(h!(Range)) {
        let mut values = headers.get(h!(Range));

        let val = values.next().ok_or(Status::RangeNotSatisfiable)?;
        if values.next().is_some() || ! val.starts_with("bytes=") {
            return res.status(Status::RangeNotSatisfiable).ok();
        }

        let slice = match val[6..].parse() {
            Ok(ByteRangesSpecifier(s, Some(e))) if e < body.len() =>
                &body[s..=e],
            Ok(ByteRangesSpecifier(s, None)) if s < body.len() =>
                &body[s..],
            _ => return res.status(Status::RangeNotSatisfiable).ok(),
        };

        super::slice_body(&mut res, slice).status(Status::PartialContent)
    } else {
        super::slice_body(&mut res, body)
    }
        .header(Header::new(h!(ETag), &*metadata.etag))
        .header(Header::new(h!(LastModified), &*metadata.modified))
        .ok()
}

mod metadata {
    use std::fs::Metadata;

    use time::Timespec;

    cfg_if! {
        if #[cfg(any(target_os = "redox", unix))] {
            use std::os::unix::fs::MetadataExt;

            pub fn id(m: &Metadata) -> u64 { m.ino() }

            pub fn mtime(m: &Metadata) -> Timespec {
                Timespec::new(m.mtime(), m.mtime_nsec() as i32)
            }
        } else if #[cfg(windows)] {
            use std::os::windows::fs::MetadataExt;

            pub fn id(m: &Metadata) -> u64 { m.creation_time() }

            pub fn mtime(m: &Metadata) -> Timespec {
                const FILE_TIME_TO_EPOCH: i64 = 116_444_736_000_000_000;
                const SEC: i64 = 10_000_000;

                // TODO: https://github.com/rust-lang/rust/issues/49048
                fn div_euc(lhs: i64, rhs: i64) -> i64 {
                    let q = lhs / rhs;
                    if lhs % rhs < 0 {
                        return if rhs > 0 { q - 1 } else { q + 1 }
                    }
                    q
                }

                fn mod_euc(lhs: i64, rhs: i64) -> i64 {
                    let r = lhs % rhs;
                    if r < 0 { return r + rhs.abs(); }
                    r
                }

                let hnsec = m.last_write_time() as i64 - FILE_TIME_TO_EPOCH;
                Timespec::new(div_euc(hnsec, SEC), mod_euc(hnsec, SEC) * 100)
            }
        } else {
            use std::time::UNIX_EPOCH;

            pub fn id(m: &Metadata) -> u64 {
                match m.created.unwrap().duration_since(UNIX_EPOCH) {
                    Ok(d) => d.as_secs(),
                    Err(e) => (e.duration().as_secs() as i64)
                        .overflowing_neg().0 as u64,
                }
            }

            pub fn mtime(m: &Metadata) -> Timespec {
                match m.modified().unwrap().duration_since(UNIX_EPOCH) {
                    Ok(d) => Timespec::new(
                        d.as_secs() as _,
                        d.subsec_nanos() as _,
                    ),
                    Err(e) => {
                        let d = e.duration();
                        let (s, ns) = (d.as_secs(), d.subsec_nanos());
                        Timespec::new(
                            if ns == 0 { -(s as i64) } else { -(s as i64) - 1 },
                            1_000_000_000 - s as i32,
                        )
                    },
                }
            }
        }
    }
}
