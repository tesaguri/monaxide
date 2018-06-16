mod topics;

pub(in bbs) use self::topics::{Topics, TopicsBuilder};

use std::borrow::Borrow;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use parking_lot::RwLock;
use rocket::http::uncased::{Uncased, UncasedStr};

use super::Topic;
use responder::Cacheable;
use setting::Settings;
use util::LinkedHashMap;

pub struct Board {
    id: Box<UncasedStr>,
    settings: Settings,
    pub(in bbs) topics: RwLock<Topics>,
}

pub struct BoardBuilder {
    id: Box<UncasedStr>,
    settings: Settings,
    topics_builder: TopicsBuilder,
}

pub type SubjectTxt = Cacheable<Vec<u8>>;

type TopicMap = LinkedHashMap<u64, Topic>;

impl Board {
    pub fn build(id: String, settings: Settings) -> BoardBuilder {
        BoardBuilder::new(id, settings)
    }

    pub fn id(&self) -> &str {
        self.id.as_str()
    }

    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    pub fn subject_txt(&self) -> Arc<SubjectTxt> {
        Arc::clone(self.topics.read().subject_txt())
    }

    fn new(id: Box<UncasedStr>, settings: Settings, topics: Topics) -> Self {
        Board {
            id,
            settings,
            topics: RwLock::new(topics),
        }
    }
}

impl Borrow<UncasedStr> for Board {
    fn borrow(&self) -> &UncasedStr {
        &self.id
    }
}

/// The hash only depends on the board id.
impl Hash for Board {
    fn hash<H: Hasher>(&self, state: &mut H) {
        Hash::hash(&self.id, state)
    }
}

/// This only compares the board ids.
impl PartialEq for Board {
    fn eq(&self, rhs: &Self) -> bool {
        PartialEq::eq(&self.id, &rhs.id)
    }
}

impl Eq for Board where Box<UncasedStr>: Eq {}

impl BoardBuilder {
    pub fn new(id: String, settings: Settings) -> Self {
        BoardBuilder {
            id: Uncased::new(id).into_boxed_uncased(),
            settings,
            topics_builder: Topics::build(),
        }
    }

    pub fn topic(&mut self, key: u64, topic: Topic) -> &mut Self {
        self.topics_builder.insert(key, topic);
        self
    }

    pub fn finish(self) -> Board {
        Board::new(self.id, self.settings, self.topics_builder.finish())
    }
}
