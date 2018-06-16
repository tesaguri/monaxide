//! Monaxide internally uses the term _topic_ to refer to a BBS thread
//! in order to avoid confusion with `std::thread`.

use std::io::{self, Read};
use std::mem;

use memchr;

pub struct Topic {
    id: u64,
    title: Box<[u8]>,
    post_count: usize,
}

impl Topic {
    #[inline]
    pub fn new(id: u64, title: Vec<u8>, post_count: usize) -> Self {
        Topic {
            id,
            title: title.into(),
            post_count,
        }
    }

    pub fn load<R: Read>(id: u64, mut src: R) -> Result<Self, io::Error> {
        let mut buf = unsafe {
            const BUF_SIZE: usize = 8 * 1024;
            let mut buf: [u8; BUF_SIZE] = mem::uninitialized();
            src.initializer().initialize(&mut buf);
            buf
        };
        let mut len;
        let mut title = Vec::new();
        let mut post_count = 0;

        macro_rules! read {
            () => {
                match src.read(&mut buf)? {
                    0 => return Ok(Topic::new(id, title, post_count)),
                    n => len = n,
                }
            };
        }

        // Read the first line that contains the title of the topic.
        let mut i = 'outer: {
            // Skip until the title:
            let mut col = 0;
            let mut lt = false;
            let mut i = 'inner: loop {
                read!();

                if lt && b'>' == buf[0] {
                    col += 1;
                    if 4 == col {
                        break 'inner 1;
                    }
                }

                for i in memchr::Memchr2::new(b'<', b'\n', &buf[..len]) {
                    if b'<' == buf[i] {
                        if len - 1 == i {
                            lt = true;
                            continue 'inner;
                        } else if b'>' == buf[i+1] {
                            col += 1;
                            if 4 == col {
                                break 'inner i+2;
                            }
                        }
                    } else { // b'\n'
                        post_count += 1;
                        break 'outer i+1;
                    }
                }
            };

            // Read the title:
            loop {
                if let Some(tlen) = memchr::memchr(b'\n', &buf[i..len]) {
                    title.extend_from_slice(&buf[i..(i+tlen)]);
                    break 'outer i + tlen + 1;
                } else {
                    title.extend_from_slice(&buf[i..len]);
                    read!();
                    i = 0;
                }
            }
        };

        loop {
            post_count += memchr::Memchr::new(b'\n', &buf[i..len]).count();
            read!();
            i = 0;
        }
    }

    #[inline]
    pub fn id(&self) -> u64 {
        self.id
    }

    #[inline]
    pub fn title(&self) -> &[u8] {
        &self.title
    }

    #[inline]
    pub fn post_count(&self) -> usize {
        self.post_count
    }

    #[inline]
    pub fn post_count_mut(&mut self) -> &mut usize {
        &mut self.post_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load() {
        // http://rio2016.5ch.net/test/read.cgi/dejima/1351397527/
        let t = Topic::load(b"\
            maji<>sage<>2012/10/28(Sun) 13:12:07.97 ID:nXycV/Aa0<>('A') ...<>('A')\n\
            maji<>sage<>2012/10/28(Sun) 15:15:21.32 ID:nXycV/Aa0<>('A' ) ...<>\n\
            !softbank221044009121.bbtec.net<>sage<>2012/10/29(Mon) 17:44:00.15 ID:dJX3cXbx0<>a\n"
        );
        assert_eq!(b"('A')", t.title);
        assert_eq!(3, t.post_count.read().unwrap());
    }
}
