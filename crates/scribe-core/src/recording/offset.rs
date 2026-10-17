use std::time::Duration;

const SECONDS_PER_MINUTE: u64 = 60;
const SECONDS_PER_HOUR: u64 = 3_600;
const MAX_CLOCK_PART_DIGITS: usize = 4;

#[must_use]
pub fn offset_as_clock(offset: Duration) -> String {
    let seconds = offset.as_secs();
    format!(
        "{:02}:{:02}:{:02}",
        seconds / SECONDS_PER_HOUR,
        seconds % SECONDS_PER_HOUR / SECONDS_PER_MINUTE,
        seconds % SECONDS_PER_MINUTE
    )
}

#[must_use]
pub fn offset_from_clock(clock: &str) -> Option<Duration> {
    let parts: Vec<&str> = clock.split(':').collect();
    let [hours, minutes, seconds] = <[&str; 3]>::try_from(parts).ok()?;
    let total = clock_part(hours)? * SECONDS_PER_HOUR
        + clock_part(minutes)? * SECONDS_PER_MINUTE
        + clock_part(seconds)?;
    Some(Duration::from_secs(total))
}

fn clock_part(part: &str) -> Option<u64> {
    let is_number = (2..=MAX_CLOCK_PART_DIGITS).contains(&part.len())
        && part.bytes().all(|byte| byte.is_ascii_digit());
    is_number.then(|| part.parse().ok())?
}
