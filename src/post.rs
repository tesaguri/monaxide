use std::borrow::Cow;

pub struct Post<'r> {
    name: Cow<'r, [u8]>,
    mail: Cow<'r, [u8]>,
    datetime: Vec<u8>,
    body: Cow<'r, [u8]>,
    title: Option<Cow<'r, [u8]>>,
}

impl<'r> Post<'r> {
    #[inline]
    pub fn new<N, M, B>(name: N, mail: M, body: B, title: Option<Cow<'r, [u8]>>) -> Self
        where N: Into<Cow<'r, [u8]>>, M: Into<Cow<'r, [u8]>>, B: Into<Cow<'r, [u8]>>
    {
        Post {
            name: name.into(),
            mail: mail.into(),
            datetime: Vec::new(),
            body: body.into(),
            title,
        }
    }
}

impl<'r> Post<'r> {
    #[inline]
    pub fn name(&self) -> &[u8] {
        &self.name
    }

    #[inline]
    pub fn name_mut(&mut self) -> &mut Vec<u8> {
        self.name.to_mut()
    }

    #[inline]
    pub fn mail(&self) -> &[u8] {
        &self.mail
    }

    #[inline]
    pub fn mail_mut(&mut self) -> &mut Vec<u8> {
        self.mail.to_mut()
    }

    #[inline]
    pub fn datetime(&self) -> &[u8] {
        &self.datetime
    }

    #[inline]
    pub fn datetime_mut(&mut self) -> &mut Vec<u8> {
        &mut self.datetime
    }

    #[inline]
    pub fn title(&self) -> Option<&[u8]> {
        self.title.as_ref().map(|t| &**t)
    }

    #[inline]
    pub fn title_mut(&mut self) -> Option<&mut Vec<u8>> {
        self.title.as_mut().map(|t| t.to_mut())
    }

    #[inline]
    pub fn body(&self) -> &[u8] {
        &self.body
    }

    #[inline]
    pub fn body_mut(&mut self) -> &mut Vec<u8> {
        self.body.to_mut()
    }
}
