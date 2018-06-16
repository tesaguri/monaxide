use std::borrow::Cow;

use typemap::ShareMap;

use super::{AfterMiddleware, BeforeMiddleware, Request};
use post::Post;
use setting::Settings;
use util::erase_lifetime;

#[derive(Default)]
pub struct Middlewares {
    before: Vec<&'static (BeforeMiddleware+Send+Sync)>,
    after: Vec<&'static (AfterMiddleware+Send+Sync)>,
    all: Vec<Box<Any+Send+Sync>>,
}

trait Any {}
impl<T: ?Sized> Any for T {}

impl Middlewares {
    #[inline]
    pub fn new() -> Self {
        Default::default()
    }

    pub fn attach<M>(&mut self, middleware: M)
        where M: BeforeMiddleware + AfterMiddleware + Send + Sync + 'static
    {
        let boxed = Box::new(middleware);
        unsafe {
            let r = erase_lifetime(boxed.as_ref());
            self.before.push(r);
            self.after.push(r);

            // This may involve a reallocation but it's sound
            // since the locations of underlying boxes are stable.
            self.all.push(boxed);
        }
    }

    pub fn attach_before<M>(&mut self, middleware: M)
        where M: BeforeMiddleware + Send + Sync + 'static
    {
        let boxed = Box::new(middleware);
        unsafe {
            self.before.push(erase_lifetime(boxed.as_ref()));
            self.all.push(boxed);
        }
    }

    pub fn attach_after<M>(&mut self, middleware: M)
        where M: AfterMiddleware + Send + Sync + 'static
    {
        let boxed = Box::new(middleware);
        unsafe {
            self.after.push(erase_lifetime(boxed.as_ref()));
            self.all.push(boxed);
        }
    }

    pub fn apply<'a, 'r, 'b, 'k>(
        &self,
        mut post: &mut Post,
        req: &Request<'a, 'r, 'b, 'k>,
        settings: &Settings,
    )
        -> Result<(), Cow<'r, [u8]>>
    {
        let mut data = ShareMap::custom();

        for m in self.before() {
            m.before(&mut data, &post, &req, &settings)?;
        }
        for m in self.after() {
            m.after(&mut post, &data, &settings)?;
        }

        Ok(())
    }

    pub fn before(&self) -> &[&(BeforeMiddleware+Send+Sync)] {
        // This method shrinks the `'static` lifetime of the references.
        &self.before
    }

    pub fn after(&self) -> &[&(AfterMiddleware+Send+Sync)] {
        &self.after
    }
}
