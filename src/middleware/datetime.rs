use std::fmt::{self, Debug, Formatter};
use std::io::Write;

use chrono::{self, Datelike, FixedOffset, LocalResult, NaiveDate, NaiveDateTime, Offset, Timelike, TimeZone, Utc};
use typemap::{Key, ShareMap};

use super::{AfterMiddleware, BeforeMiddleware, Request, Result};
use post::Post;
use setting::{self, Settings};

pub struct DateTime<Tz=Jst>(pub Tz);

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Jst;

impl<Tz: TimeZone+'static> Key for DateTime<Tz> {
    type Value = chrono::DateTime<Tz>;
}

impl<Tz> DateTime<Tz> {
    pub fn new(tz: Tz) -> Self where Tz: TimeZone {
        DateTime(tz)
    }
}

impl DateTime {
    pub fn with_jst() -> Self {
        DateTime(Jst)
    }
}

impl DateTime<Utc> {
    pub fn with_utc() -> DateTime<Utc> {
        DateTime(Utc)
    }
}

impl<Tz: TimeZone+'static> BeforeMiddleware for DateTime<Tz> where Tz::Offset: Send+Sync {
    fn before<'a, 'r, 'b, 'k>(&self, data: &mut ShareMap, _: &Post, _: &Request<'a, 'r, 'b, 'k>, _: &Settings)
        -> Result<'r, ()>
    {
        let now = self.0.from_utc_datetime(&Utc::now().naive_utc());
        data.insert::<Self>(now);
        Ok(())
    }
}

impl<Tz: TimeZone> AfterMiddleware for DateTime<Tz> {
    fn after(&self, post: &mut Post, data: &ShareMap, setting: &Settings) -> Result<'static, ()> {
        const WEEKDAYS: [[u8; 2]; 7] = [
            *b"\x93\xFA",  // 日
            *b"\x8C\x8E",  // 月
            *b"\x89\xCE",  // 火
            *b"\x90\x85",  // 水
            *b"\x96\xD8",  // 木
            *b"\x8B\xE0",  // 金
            *b"\x93\x79"]; // 土

        if let Some(dt) = data.get::<DateTime>() {
            let (date, time) = (dt.date(), dt.time());
            let (y, mon, d, wday) = (date.year(), date.month(), date.day(), date.weekday() as usize);
            let wday = setting.get::<setting::common::YmdWeeks>().map_or_else(
                || &WEEKDAYS[wday] as &[u8],
                |wdays| &wdays[wday] as &[u8],
            );
            let (h, m, mut s, mut cs) = (time.hour(), time.minute(), time.second(), time.nanosecond() / 10_000_000);
            if cs >= 100 { cs -= 100; s += 1; } // leap second

            let dt = post.datetime_mut();
            // "2000/01/01(土) 12:51:48.97"
            super::reserve_and_delimit(dt, 24 + wday.len());
            write!(dt, "{:04}/{:02}/{:02}(", y, mon, d).unwrap();
            dt.extend_from_slice(wday);
            write!(dt, ") {:02}:{:02}:{:02}.{:02}", h, m, s, cs).unwrap();
        }

        Ok(())
    }
}

impl TimeZone for Jst {
    type Offset = Self;

    fn from_offset(_offset: &Self) -> Self {
        Jst
    }

    fn offset_from_local_date(&self, _local: &NaiveDate) -> LocalResult<Self> {
        LocalResult::Single(Jst)
    }

    fn offset_from_local_datetime(&self, _local: &NaiveDateTime) -> LocalResult<Self> {
        LocalResult::Single(Jst)
    }

    fn offset_from_utc_date(&self, _utc: &NaiveDate) -> Self {
        Jst
    }

    fn offset_from_utc_datetime(&self, _utc: &NaiveDateTime) -> Self {
        Jst
    }
}

impl Offset for Jst {
    fn fix(&self) -> FixedOffset {
        FixedOffset::east(9 * 60*60)
    }
}

impl Debug for Jst {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str("JST")
    }
}
