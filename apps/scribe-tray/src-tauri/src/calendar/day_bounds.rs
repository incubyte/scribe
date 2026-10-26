use chrono::{DateTime, FixedOffset, NaiveDate, NaiveTime, TimeZone};
use scribe_core::calendar::DayBounds;

pub fn today_bounds<Tz: TimeZone>(now: &DateTime<Tz>) -> Option<DayBounds> {
    let today = now.date_naive();
    let timezone = now.timezone();
    Some(DayBounds {
        start: midnight_of(&timezone, today)?,
        end: midnight_of(&timezone, today.succ_opt()?)?,
    })
}

fn midnight_of<Tz: TimeZone>(timezone: &Tz, date: NaiveDate) -> Option<DateTime<FixedOffset>> {
    timezone
        .from_local_datetime(&date.and_time(NaiveTime::MIN))
        .earliest()
        .map(|midnight| midnight.fixed_offset())
}
