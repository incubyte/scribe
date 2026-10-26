use chrono::{DateTime, FixedOffset, SecondsFormat, Utc};
use scribe_core::calendar::DayBounds;
use url::Url;

use crate::authority::CALENDAR_VIEW_URL;

pub const PAGE_SIZE: &str = "100";
pub const SELECTED_FIELDS: &str = "id,subject,start,end,organizer,attendees,isAllDay,isCancelled,onlineMeetingProvider,type,responseStatus,showAs";

#[must_use]
pub fn calendar_view_url(day: &DayBounds) -> Url {
    let mut url = Url::parse(CALENDAR_VIEW_URL).expect("the calendar view address is a valid URL");
    url.query_pairs_mut()
        .append_pair("startDateTime", &utc_text(day.start))
        .append_pair("endDateTime", &utc_text(day.end))
        .append_pair("$top", PAGE_SIZE)
        .append_pair("$select", SELECTED_FIELDS);
    url
}

fn utc_text(moment: DateTime<FixedOffset>) -> String {
    moment
        .with_timezone(&Utc)
        .to_rfc3339_opts(SecondsFormat::Secs, true)
}
