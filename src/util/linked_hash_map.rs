use super::unsafe_linked_list::{self, UnsafeLinkedList, NodeHandle};

use std::collections::hash_map::{self, HashMap};
use std::hash::Hash;

/// A hash map with a preserved LIFO order of entries backed by a linked list.
///
/// It supports inserting, removing and _bumping_ entries in constant time.
///
/// # Example
///
/// ```
/// let mut map = LinkedHashMap::new();
/// map.insert(2, "2");
/// map.insert(3, "3");
/// map.insert(1, "1");
/// map.remove(3);
/// map.insert(0, "0");
/// assert!(map.iter().eq([(0, &"0"), (1, &"1"), (2, &"2")].iter().cloned()));
/// ```
pub struct LinkedHashMap<K, V> {
    list: UnsafeLinkedList<(K, V)>,
    map: HashMap<K, NodeHandle<(K, V)>>,
}

/// An iterator over values of a `LinkedHashMap`.
pub struct Iter<'a, K: 'a, V: 'a> {
    inner: unsafe_linked_list::Iter<'a, (K, V)>,
}

pub enum Entry<'a, K: 'a, V: 'a> {
    Occupied(OccupiedEntry<'a, K, V>),
    Vacant(VacantEntry<'a, K, V>),
}

pub struct OccupiedEntry<'a, K: 'a, V: 'a> {
    inner: hash_map::OccupiedEntry<'a, K, NodeHandle<(K, V)>>,
    list: &'a mut UnsafeLinkedList<(K, V)>,
}

pub struct VacantEntry<'a, K: 'a, V: 'a> {
    inner: hash_map::VacantEntry<'a, K, NodeHandle<(K, V)>>,
    list: &'a mut UnsafeLinkedList<(K, V)>,
}

impl<K: Hash+Eq+Copy, V> LinkedHashMap<K, V> {
    pub fn new() -> Self {
        LinkedHashMap {
            list: UnsafeLinkedList::new(),
            map: HashMap::new(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        LinkedHashMap {
            list: UnsafeLinkedList::new(),
            map: HashMap::with_capacity(capacity),
        }
    }

    pub fn entry(&mut self, k: K) -> Entry<K, V> {
        match self.map.entry(k) {
            hash_map::Entry::Occupied(inner) =>
                Entry::Occupied(OccupiedEntry { inner, list: &mut self.list }),
            hash_map::Entry::Vacant(inner) =>
                Entry::Vacant(VacantEntry { inner, list: &mut self.list }),
        }
    }

    /// Attempts to insert an entry to the map.
    ///
    /// This does not overwrite an existing entry.
    /// Returns back the value if it wasn't inserted.
    ///
    /// # Example
    ///
    /// ```
    /// let mut map = LinkedHashMap::new();
    /// assert!(map.insert(1, "1").is_none());
    /// assert!(map.insert(4, "2").is_none());
    /// assert!(map.insert(9, "3").is_none());
    /// assert_eq!("4", map.insert(4, "4").unwrap());
    ///
    /// assert_eq!(Some(&"1"), map.get(1));
    /// assert_eq!(Some(&"2"), map.get(4));
    /// assert_eq!(Some(&"3"), map.get(9));
    /// ```
    pub fn insert(&mut self, k: K, v: V) -> Option<V> {
        if let hash_map::Entry::Vacant(e) = self.map.entry(k) {
            e.insert(self.list.push_front((k, v)));
            None
        } else {
            Some(v)
        }
    }

    pub fn get(&self, k: K) -> Option<&V> {
        self.map.get(&k).map(|node| unsafe { &self.list.get(node).1 })
    }

    pub fn get_mut(&mut self, k: K) -> Option<&mut V> {
        let list = &mut self.list;
        self.map.get(&k).map(move |node| unsafe { &mut list.get_mut(node).1 })
    }

    pub fn contains_key(&self, k: K) -> bool {
        self.map.contains_key(&k)
    }

    /// Removes an entry from the map.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut map = LinkedHashMap::new();
    /// map.insert(0, 2);
    /// map.insert(1, 1);
    /// map.insert(2, 0);
    /// assert!(map.iter().eq([(2, &0), (1, &1), (0, &2)].iter().cloned()));
    ///
    /// map.remove(1);
    /// map.remove(0);
    /// assert!(map.iter().eq([(2, &0)].iter().cloned()));
    /// ```
    pub fn remove(&mut self, k: K) -> Option<V> {
        self.map.remove(&k).map(|n| {
            unsafe {
                self.list.remove(n).1
            }
        })
    }

    /// Bumps an entry to the front of the list.
    ///
    /// # Example
    ///
    /// ```
    /// let mut map = LinkedHashMap::new();
    /// map.insert(1, "1");
    /// map.insert(2, "2");
    /// map.insert(3, "3");
    /// assert!(map.iter().eq([(3, &"3"), (2, &"2"), (1, &"1")].iter().cloned()));
    ///
    /// map.bump(1);
    /// assert!(map.iter().eq([(1, &"1"), (3, &"3"), (2, &"2")].iter().cloned()));
    /// ```
    pub fn bump(&mut self, k: K) -> bool {
        if let Some(n) = self.map.get(&k) {
            unsafe {
                self.list.bump(n);
            }
            true
        } else {
            false
        }
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }
}

impl<K: Copy, V> LinkedHashMap<K, V> {
    /// Returns an iterator visiting all key-value pairs from the front of
    /// the list.
    ///
    /// # Example
    ///
    /// ```
    /// let mut map = LinkedHashMap::new();
    /// map.insert(7, "a");
    /// map.insert(11, "b");
    /// map.insert(3, "c");
    /// assert!(map.iter().eq([(3, &"c"), (11, &"b"), (7, &"a")].iter().cloned()));
    /// ```
    pub fn iter(&self) -> Iter<K, V> {
        Iter { inner: self.list.iter() }
    }
}

unsafe impl<K, V> Send for LinkedHashMap<K, V> where K: Send, V: Send {}
unsafe impl<K, V> Sync for LinkedHashMap<K, V> where K: Sync, V: Sync {}

impl<'a, K: Copy, V> IntoIterator for &'a LinkedHashMap<K, V> {
    type Item = (K, &'a V);
    type IntoIter = Iter<'a, K, V>;

    fn into_iter(self) -> Iter<'a, K, V> {
        self.iter()
    }
}

impl<'a, K: Copy, V> Iterator for Iter<'a, K, V> {
    type Item = (K, &'a V);

    fn next(&mut self) -> Option<(K, &'a V)> {
        self.inner.next().map(|&(k, ref v)| (k, v))
    }
}
