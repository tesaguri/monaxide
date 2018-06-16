use std::borrow::Cow;
use std::ops::Deref;

use percent_encoding::percent_decode;
use rocket::http::RawStr;
use rocket::request::{FromFormValue, FromParam};

/// /[A-Za-z\d]+/
#[derive(Clone, Copy)]
pub struct AlphaNum<'v>(&'v str);

/// /\d+/
#[derive(Clone, Copy)]
pub struct Digits<'v> {
    pub number: u64,
    raw: &'v str,
}

/// HTML-escaped byte string
pub struct Escaped<'v>(Cow<'v, [u8]>);

impl<'v> AlphaNum<'v> {
    pub fn as_str(&self) -> &'v str {
        self.0
    }
}

impl<'v> Digits<'v> {
    pub fn as_str(&self) -> &'v str {
        self.raw
    }

    /// # Safety
    ///
    /// The caller must ensure that `number.to_string() == raw`.
    #[doc(hidden)]
    pub unsafe fn new_unchecked(number: u64, raw: &'v str) -> Self {
        Digits { number, raw }
    }
}

impl<'v> FromFormValue<'v> for AlphaNum<'v> {
    type Error = &'v RawStr;

    fn from_form_value(v: &'v RawStr) -> Result<Self, &'v RawStr> {
        if v.len() > 0 && v.as_bytes().iter().all(|&c| is_alphanum(c)) {
            Ok(AlphaNum(v))
        } else {
            Err(v)
        }
    }
}

impl<'v> FromParam<'v> for AlphaNum<'v> {
    type Error = &'v RawStr;

    fn from_param(v: &'v RawStr) -> Result<Self, &'v RawStr> {
        FromFormValue::from_form_value(v)
    }
}

impl<'v> FromFormValue<'v> for Digits<'v> {
    type Error = &'v RawStr;

    fn from_form_value(v: &'v RawStr) -> Result<Self, &'v RawStr> {
        v.parse()
            .map(|number| Digits { number, raw: v })
            .or(Err(v))
    }
}

impl<'v> FromFormValue<'v> for Escaped<'v> {
    type Error = !;

    fn from_form_value(v: &'v RawStr) -> Result<Self, !> {
        fn html_escape(src: Vec<u8>) -> Vec<u8> {
            let mut ret: Option<Vec<u8>> = None;

            for (i, &c) in src.iter().enumerate() {
                let esc: &[u8] = match c {
                    // These rules are not enough for general HTML escaping,
                    // but this is what 2channel does, so we adopt them
                    // as they are for compatibility.
                    b'"' => b"&quot;",
                    b'<' => b"&lt;",
                    b'>' => b"&gt;",
                    _ => {
                        ret.as_mut().map(|v| v.push(c));
                        continue;
                    },
                };
                ret.get_or_insert_with(|| {
                    let mut vec = Vec::with_capacity(src.len() * 2);
                    vec.extend_from_slice(&src[..i]);
                    vec
                }).extend_from_slice(esc);
            }

            ret.unwrap_or(src)
        }

        let cow = percent_decode(v.as_bytes())
            .if_any()
            .map(|owned| html_escape(owned).into())
            .unwrap_or_else(|| v.as_bytes().into()); // TODO: escaping
        Ok(Escaped(cow))
    }
}

impl<'v> AsRef<str> for AlphaNum<'v> {
    fn as_ref(&self) -> &str {
        self.0
    }
}

impl<'v> AsRef<str> for Digits<'v> {
    fn as_ref(&self) -> &str {
        self.raw
    }
}

impl<'v> AsRef<[u8]> for Escaped<'v> {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl<'v> Deref for AlphaNum<'v> {
    type Target = str;

    fn deref(&self) -> &str {
        self.as_ref()
    }
}

impl<'v> Deref for Escaped<'v> {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        self.as_ref()
    }
}

impl<'v> From<Escaped<'v>> for Cow<'v, [u8]> {
    fn from(e: Escaped<'v>) -> Self {
        e.0
    }
}

impl<'v> Deref for Digits<'v> {
    type Target = str;

    fn deref(&self) -> &str {
        self.as_ref()
    }
}

pub fn is_alphanum(c: u8) -> bool {
    (b'a' <= c && c <= b'z') || (b'A' <= c && c <= b'Z') || is_digit(c)
}

pub fn is_digit(c: u8) -> bool {
    b'0' <= c && c <= b'9'
}
