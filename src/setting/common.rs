use super::Setting;

macro_rules! from_raw {
    (u32) => {
        #[inline]
        fn from_raw(raw: &[u8]) -> Option<u32> {
            unsafe {
                str::parse(::std::str::from_utf8_unchecked(raw)).ok()
            }
        }
    };
    (bool) => {
        #[inline]
        fn from_raw(raw: &[u8]) -> Option<bool> {
            match raw {
                b"checked" => Some(true),
                b"" => Some(false),
                _ => None,
            }
        }
    };
    (String) => {
        #[inline]
        fn from_raw(raw: &[u8]) -> Option<String> {
            String::from_utf8(raw.to_owned()).ok()
        }
    };
    (Vec) => {
        #[inline]
        fn from_raw(raw: &[u8]) -> Option<Vec<u8>> {
            Some(raw.to_owned())
        }
    };
}

#[macro_export]
macro_rules! mona_settings {
    ($name:ident($key:expr) -> Vec<u8>; $($rest:tt)*) => {
        pub enum $name {}

        impl $crate::setting::Setting for $name {
            const KEY: &'static str = $key;
            type Value = Vec<u8>;

            from_raw!(Vec);
        }

        mona_settings! { $($rest)* }
    };
    ($name:ident($key:expr) -> $value:ident; $($rest:tt)*) => {
        pub enum $name {}

        impl $crate::setting::Setting for $name {
            const KEY: &'static str = $key;
            type Value = $value;

            from_raw!($value);
        }

        mona_settings! { $($rest)* }
    };
    () => ();
}

mona_settings! {
    Title("BBS_TITLE") -> Vec<u8>;
    NonameName("BBS_NONAME_NAME") -> Vec<u8>;
    LineNumber("BBS_LINE_NUMBER") -> u32;
    SubjectCount("BBS_SUBJECT_COUNT") -> u32;
    NameCount("BBS_NAME_COUNT") -> u32;
    MailCount("BBS_MAIL_COUNT") -> u32;
    MessageCount("BBS_MESSAGE_COUNT") -> u32;
    ForceId("BBS_FORCE_ID") -> bool;
    NoId("BBS_NO_ID") -> bool;
    Heisa("BBS_HEISA") -> bool;
}

pub enum Adult {}
pub enum Unicode {}
pub enum YmdWeeks {}

impl Setting for Adult {
    const KEY: &'static str = "BBS_ADULT";
    type Value = bool;

    #[inline]
    fn from_raw(raw: &[u8]) -> Option<bool> {
        match raw {
            b"1" => Some(true),
            b"0" => Some(false),
            _ => None,
        }
    }
}

impl Setting for Unicode {
    const KEY: &'static str = "BBS_UNICODE";
    type Value = bool;

    #[inline]
    fn from_raw(raw: &[u8]) -> Option<bool> {
        match raw {
            b"pass" => Some(true),
            b"change" => Some(false),
            _ => None,
        }
    }
}

impl Setting for YmdWeeks {
    const KEY: &'static str = "BBS_YMD_WEEKS";
    type Value = [Box<[u8]>; 7];

    fn from_raw(raw: &[u8]) -> Option<[Box<[u8]>; 7]> {
        let mut iter = raw.split(|&c| b'/' == c);

        iter.next().and_then(|sun|
        iter.next().and_then(|mon|
        iter.next().and_then(|tue|
        iter.next().and_then(|wed|
        iter.next().and_then(|thu|
        iter.next().and_then(|fri|
        iter.next().map     (|sat| [
            Box::from(sun),
            Box::from(mon),
            Box::from(tue),
            Box::from(wed),
            Box::from(thu),
            Box::from(fri),
            Box::from(sat),
        ])))))))
    }
}
