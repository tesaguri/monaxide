mod linked_hash_map;
mod unsafe_linked_list;

pub use self::linked_hash_map::LinkedHashMap;

pub unsafe fn erase_lifetime<'a, T: ?Sized>(t: &T) -> &'a T {
    ::std::mem::transmute::<&T, &'a T>(t)
}
