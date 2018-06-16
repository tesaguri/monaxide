use std::borrow::Cow;
use std::io::{self, Write};
use std::path::PathBuf;

use rocket::outcome::Outcome::*;
use rocket::request::{Form, FromRequest, Outcome, Request};
use rocket::response::status::Created;

use bbs::Bbs;
use middleware;
use post::Post;
use responder::Bytes;
use validator::{AlphaNum, Digits, Escaped};

#[allow(non_snake_case)]
#[derive(FromForm)]
pub struct BbsForm<'r> {
    bbs: AlphaNum<'r>,
    key: Option<Digits<'r>>,
    // time: u64,
    // submit: &'r str,
    subject: Option<Escaped<'r>>,
    FROM: Escaped<'r>,
    mail: Escaped<'r>,
    MESSAGE: Escaped<'r>,
}

pub struct RequestFromRequest<'a, 'r: 'a>(&'a Request<'r>);

#[post("/bbs.cgi", data="<form>")]
pub fn post<'a, 'r>(form: Form<'r, BbsForm<'r>>, bbs: &'r Bbs, req: RequestFromRequest<'a, 'r>)
    -> Result<Created<Bytes<'static>>, Cow<'r, [u8]>>
{
    let form = form.get();

    let brd = bbs.board(&form.bbs)
        .ok_or(b"Board not found" as &[u8])?;

    let mut path = PathBuf::from("dat".to_owned());
    path.push(&*form.bbs);

    let key_str;
    let (key, dat) = if let Some(key) = form.key {
        let t = brd.topic_mut(key.number)
            .ok_or(b"Thread not found" as &[u8])?;
        (key, t.into_dat())
    } else if let Some(title) = form.subject.as_ref() {
        let t = brd.create_topic(title.to_vec());
        let key = unsafe {
            key_str = t.id().to_string();
            Digits::new_unchecked(t.id(), &key_str)
        };
        (key, t.into_dat())
    } else {
        return Err((b"Either `key` or `subject` parameter is required" as &[u8]).into());
    };

    let mut dat = dat.unwrap_or_else(|e| {
        panic!(
            "failed to open an existing thread, {}/{}: {:?}",
            &*form.bbs, &*key, e
        );
    });

    let mut post = Post::new(
        &*form.FROM,
        &*form.mail,
        &*form.MESSAGE,
        form.subject.as_ref().map(|s| (**s).into()),
    );
    let req = middleware::Request::new(form.bbs, key, req.0);

    bbs.apply_middlewares(&mut post, &req, &brd.settings())?;

    write_dat_line(&mut dat, &post)
        .unwrap_or_else(|e| {
            panic!("failed to write to a file, {:?}: {:?}", &path, e);
        });
    dat.increment_post_count();

    // "書き込みました。"
    const SUCCESS: &[u8] =
        b"\x8F\x91\x82\xAB\x8D\x9E\x82\xDD\x82\xDC\x82\xB5\x82\xBD\x81\x42";
    let url = format!("read.cgi/{}/{}/", &*form.bbs, &*key); // TODO: Post #
    Ok(Created(url, Some(Bytes(SUCCESS))))
}

impl<'a, 'r> FromRequest<'a, 'r> for RequestFromRequest<'a, 'r> {
    type Error = !;

    fn from_request(req: &'a Request<'r>) -> Outcome<Self, !> {
        Success(RequestFromRequest(req))
    }
}

fn write_dat_line<W: Write>(mut dat: W, post: &Post) -> io::Result<()> {
    // name<>mail<>datetime<> body <>\n
    let mut line = Vec::with_capacity(
        post.name().len() + post.mail().len() + post.datetime().len() + post.body().len() + 11
    );

    line.extend_from_slice(&post.name());
    line.extend_from_slice(b"<>");
    line.extend_from_slice(&post.mail());
    line.extend_from_slice(b"<>");
    line.extend_from_slice(&post.datetime());
    line.extend_from_slice(b"<> ");
    line.extend_from_slice(&post.body());
    line.extend_from_slice(b" <>\n");

    dat.write_all(&line)
}
