pub mod board;
pub mod topic;

pub use self::board::Board;
pub use self::topic::Topic;

use std::borrow::Cow;
use std::collections::HashSet;
use std::ffi::OsString;
use std::fs::{self, File};
use std::io;
use std::ops::{Deref, DerefMut};
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::result;
use std::str;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use owning_ref::{OwningRef, OwningRefMut};
use parking_lot::{RwLockReadGuard, RwLockWriteGuard};
use rocket::http::uncased::UncasedStr;
use rocket::request::{FromRequest, Outcome, Request, State};

use self::board::{SubjectTxt, Topics};
use middleware::{self, BeforeMiddleware, AfterMiddleware, Middlewares};
use post::Post;
use setting::Settings;
use validator;

pub struct Bbs {
    boards: HashSet<Board>,
    middlewares: Middlewares,
    workspace: Box<Path>,
}

#[derive(Clone, Copy)]
pub struct BoardRef<'a> {
    inner: &'a Board,
    bbs: &'a Bbs,
}

pub struct TopicRef<'a> {
    inner: OwningRef<RwLockReadGuard<'a, Topics>, Topic>,
    board: &'a BoardRef<'a>,
}

pub struct TopicMut<'a> {
    inner: OwningRefMut<RwLockWriteGuard<'a, Topics>, Topic>,
    board: &'a BoardRef<'a>,
}

pub struct DatRef<'a> {
    inner: File,
    topic: TopicMut<'a>,
}

impl Bbs {
    pub fn new() -> Result<Self, io::Error> {
        Bbs::with_workspace(".")
    }

    pub fn with_workspace<P>(workspace: P)-> Result<Self, io::Error>
        where P: AsRef<Path>
    {
        Bbs::_with_workspace(workspace.as_ref())
    }

    fn _with_workspace(workspace: &Path) -> Result<Self, io::Error> {
        let mut boards = HashSet::new();
        for brd_ent in fs::read_dir(workspace)? {
            let brd_ent = brd_ent?;
            let mut path = brd_ent.path();
            if ! path.is_dir() { continue; }

            const MISSING_FNAME: &str =
                "`DirEntry.path().file_name()` returned a `None`";
            let board_id = {
                let n = path.file_name().expect(MISSING_FNAME).as_bytes();
                unsafe {
                    if ! n.iter().all(|&c| validator::is_alphanum(c)) {
                        continue;
                    }
                    String::from_utf8_unchecked(n.to_owned())
                }
            };

            path.push("SETTING.TXT");
            let settings = if path.exists() {
                Settings::load(&File::open(&path)?)?
            } else {
                Settings::empty()
            };

            path.set_file_name("dat");
            fs::create_dir_all(&path)?;
            let mut builder = Board::build(board_id, settings);
            for ent in fs::read_dir(&path)? {
                let ent = ent?;
                let p = ent.path();
                if ! p.is_file() { continue; }
                let n = p.file_name().expect(MISSING_FNAME).as_bytes();
                if ! n.ends_with(b".dat") { continue; }
                let key = match ::atoi::atoi(&n[0..(n.len()-4)]) {
                    Some(::checked::Checked(Some(key))) => key,
                    _ => continue,
                };
                let topic = Topic::load(key, File::open(&p)?)?;
                builder.topic(key, topic);
            }

            boards.insert(builder.finish());
        }

        Ok(Bbs {
            boards,
            middlewares: Middlewares::new(),
            workspace: workspace.to_owned().into_boxed_path(),
        })
    }

    pub fn attach<M>(&mut self, middleware: M) -> &mut Self
        where M: BeforeMiddleware + AfterMiddleware + Send + Sync + 'static
    {
        self.middlewares.attach(middleware);
        self
    }

    pub fn attach_before<M>(&mut self, middleware: M) -> &mut Self
        where M: BeforeMiddleware + Send + Sync + 'static
    {
        self.middlewares.attach_before(middleware);
        self
    }

    pub fn attach_after<M>(&mut self, middleware: M) -> &mut Self
        where M: AfterMiddleware + Send + Sync + 'static
    {
        self.middlewares.attach_after(middleware);
        self
    }

    pub fn before_middlewares(&self) -> &[&(BeforeMiddleware+Send+Sync)] {
        self.middlewares.before()
    }

    pub fn after_middlewares(&self) -> &[&(AfterMiddleware+Send+Sync)] {
        self.middlewares.after()
    }

    pub fn apply_middlewares<'a, 'r, 'b, 'k>(
        &self,
        post: &mut Post,
        req: &middleware::Request<'a, 'r, 'b, 'k>,
        settings: &Settings,
    )
        -> result::Result<(), Cow<'r, [u8]>>
    {
        self.middlewares.apply(post, req, settings)
    }

    #[inline]
    pub fn board(&self, name: &str) -> Option<BoardRef> {
        self.boards.get(UncasedStr::new(name)).map(|inner| BoardRef {
            inner,
            bbs: self,
        })
    }
}

impl<'a, 'r> FromRequest<'a, 'r> for &'r Bbs {
    type Error = ();

    fn from_request(req: &'a Request<'r>) -> Outcome<Self, ()> {
        req.guard::<State<Bbs>>().map(|s| s.inner())
    }
}

impl<'a> BoardRef<'a> {
    pub fn id(&self) -> &'a str {
        self.inner.id()
    }

    pub fn settings(&self) -> &'a Settings {
        self.inner.settings()
    }

    pub fn subject_txt(&self) -> Arc<SubjectTxt> {
        self.inner.subject_txt()
    }

    pub fn topic(&self, key: u64) -> Option<TopicRef> {
        let guard = self.inner.topics.read();
        OwningRef::new(guard)
            .try_map(|topics| topics.get(key).ok_or(()))
            .ok()
            .map(|inner| TopicRef { inner, board: self })
    }

    pub fn topic_mut(&'a self, key: u64) -> Option<TopicMut<'a>> {
        let guard = self.inner.topics.write();
        OwningRefMut::new(guard)
            .try_map_mut(|topics| topics.get_mut(key).ok_or(()))
            .ok()
            .map(|inner| TopicMut { inner, board: self })
    }

    pub fn create_topic(&'a self, title: Vec<u8>) -> TopicMut<'a> {
        let mut guard = self.inner.topics.write();
        let mut id = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time before the UNIX epoch")
            .as_secs();
        while guard.contains_key(id) { id += 1; }
        let _ret = guard.insert(Topic::new(id, title, 0));
        debug_assert!(_ret.is_none());

        let inner = OwningRefMut::new(guard)
            .map_mut(|topics| topics.get_mut(id).unwrap());
        TopicMut { inner, board: self }
    }

    pub fn bbs(&self) -> &'a Bbs {
        &self.bbs
    }
}

impl<'a> Deref for BoardRef<'a> {
    type Target = Board;

    fn deref(&self) -> &Board {
        self.inner
    }
}

impl<'a> Deref for TopicRef<'a> {
    type Target = Topic;

    fn deref(&self) -> &Topic {
        &self.inner
    }
}

impl<'a> TopicMut<'a> {
    pub fn into_dat(self) -> io::Result<DatRef<'a>> {
        let mut path = OsString::with_capacity(
            self.board.bbs.workspace.as_os_str().len()
            + self.board.id().len()
            + "//dat/0000000000.dat".len()
        );
        path.push(&*self.board.bbs.workspace);
        let mut path: PathBuf = path.into();
        path.push(self.board.id());
        path.push("dat");
        path.push(format!("{}.dat", self.id()));

        let inner = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;

        Ok(DatRef { inner, topic: self })
    }
}

impl<'a> Deref for TopicMut<'a> {
    type Target = Topic;

    fn deref(&self) -> &Topic {
        &self.inner
    }
}

impl<'a> DerefMut for TopicMut<'a> {
    fn deref_mut(&mut self) -> &mut Topic {
        &mut self.inner
    }
}

impl<'a> DatRef<'a> {
    pub fn increment_post_count(&mut self) {
        *self.post_count_mut() += 1;
        self.reset_txt(true);
    }

    fn post_count_mut(&mut self) -> &mut usize {
        self.topic.post_count_mut()
    }

    fn reset_txt(&mut self, addition: bool) {
        unsafe {
            // This is sound because `Topics::reset_txt` does not change
            // location of `Topic`s.
            let topics = OwningRefMut::owner(&self.topic.inner);
            (*(&**topics as *const Topics as *mut Topics)).reset_txt(addition);
        }
    }
}

impl<'a> io::Write for DatRef<'a> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        io::Write::write(&mut self.inner, buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        io::Write::flush(&mut self.inner)
    }
}
