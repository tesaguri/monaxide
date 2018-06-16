use std::any::TypeId;
use std::hash::{Hash, Hasher};
use std::io::Write;
use std::mem;
use std::sync::Arc;

use lazy_init::LazyTransform;

use super::{TopicMap, SubjectTxt};
use bbs::Topic;
use responder::{Cacheable, Metadata};
use util::LinkedHashMap;

pub struct Topics {
    map: TopicMap,
    subject_txt: LazyTransform<SubjectTxt, Arc<SubjectTxt>>,
}

pub struct TopicsBuilder {
    map: TopicMap,
}

struct Id;

// "TTTTTTTTTT.dat<>TITLE (NNNN)\n"
// len = 24 + len(TITLE)
const STANDARD_LINE_LEN: usize = 128;

impl Topics {
    pub fn build() -> TopicsBuilder {
        TopicsBuilder::new()
    }

    pub fn get(&self, key: u64) -> Option<&Topic> {
        self.map.get(key)
    }

    pub fn get_mut(&mut self, key: u64) -> Option<&mut Topic> {
        self.map.get_mut(key)
    }

    pub fn contains_key(&mut self, key: u64) -> bool {
        self.map.contains_key(key)
    }

    pub fn insert(&mut self, topic: Topic) -> Option<Topic> {
        let ret = self.map.insert(topic.id(), topic);
        if ret.is_none() {
            self.reset_txt(true);
        }
        ret
    }

    pub fn remove(&mut self, key: u64) -> Option<Topic> {
        let ret = self.map.remove(key);
        if ret.is_some() { self.reset_txt(false); }
        ret
    }

    pub fn subject_txt(&self) -> &Arc<SubjectTxt> {
        self.subject_txt.get_or_create(|mut txt| {
            self.make_txt(&mut txt);
            Arc::new(txt)
        })
    }

    pub fn reset_txt(&mut self, addition: bool) {
        // Try to reuse the buffer:
        let dummy = LazyTransform::default();
        let txt = mem::replace(&mut self.subject_txt, dummy)
            .into_inner()
            .map(|arc| match Arc::try_unwrap(arc) {
                Ok(txt) => txt,
                Err(arc) => { // create new buffer
                    let cap = arc.body().len() +
                        if addition { STANDARD_LINE_LEN } else { 0 };
                    let buf = Vec::with_capacity(cap);
                    Cacheable::new(buf, Metadata::now(Id::in_u64()))
                },
            })
            .unwrap_or_else(|txt| txt);
        mem::replace(&mut self.subject_txt, LazyTransform::new(txt));
    }

    fn make_txt(&self, txt: &mut SubjectTxt) {
        {
            let mut vec = txt.body_mut();

            vec.clear();

            for (k, t) in &self.map {
                // "TTTTTTTTTT.dat<>TITLE (NNNN)\n"
                write!(&mut vec, "{}", k).unwrap();
                vec.extend_from_slice(b".dat<>");
                vec.extend_from_slice(t.title());
                vec.extend_from_slice(b" (");
                write!(&mut vec, "{}", t.post_count()).unwrap();
                vec.extend_from_slice(b")\n");
            }
        }

        txt.modify(Id::in_u64());
    }
}

impl TopicsBuilder {
    pub fn new() -> Self {
        TopicsBuilder {
            map: LinkedHashMap::new(),
        }
    }

    pub fn insert(&mut self, key: u64, topic: Topic) -> &mut Self {
        self.map.insert(key, topic);
        self
    }

    pub fn finish(self) -> Topics {
        Topics {
            map: self.map,
            subject_txt: LazyTransform::new(SubjectTxt::default()),
        }
    }
}

impl Id {
    fn in_u64() -> u64 {
        struct IdentityHasher(u64);
        impl Hasher for IdentityHasher {
            fn finish(&self) -> u64 { self.0 }
            fn write_u64(&mut self, n: u64) { self.0 = n; }
            // Not likely to be called:
            fn write(&mut self, bytes: &[u8]) {
                for &b in bytes {
                    self.0 = (self.0 << 8) | b as u64;
                }
            }
        }

        let mut h = IdentityHasher(0);
        TypeId::of::<Self>().hash(&mut h);
        h.finish()
    }
}
