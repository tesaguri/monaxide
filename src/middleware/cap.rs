use typemap::Key;

pub struct Cap;

impl Key for Cap {
    type Value = Box<[u8]>;
}
