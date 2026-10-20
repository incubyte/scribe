use std::fmt;
use std::time::Duration;

use crate::recording::{offset_as_clock, offset_from_clock};

const EXTENSION: &str = ".png";
const PART_SEPARATOR: &str = "-";
const CLOCK_SEPARATOR: &str = ":";
const CLOCK_PARTS: usize = 3;
const FIRST_SUFFIX: u16 = 2;
const LAST_SUFFIX: u16 = 99;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ScreenshotName {
    offset_seconds: u64,
    suffix: Option<u16>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("not a valid screenshot name")]
pub struct InvalidScreenshotName;

impl ScreenshotName {
    #[must_use]
    pub fn at(offset: Duration) -> ScreenshotName {
        ScreenshotName {
            offset_seconds: offset.as_secs(),
            suffix: None,
        }
    }

    pub fn candidates(offset: Duration) -> impl Iterator<Item = ScreenshotName> {
        let first = ScreenshotName::at(offset);
        let suffixed = (FIRST_SUFFIX..=LAST_SUFFIX).map(move |suffix| ScreenshotName {
            suffix: Some(suffix),
            ..ScreenshotName::at(offset)
        });
        std::iter::once(first).chain(suffixed)
    }

    pub fn parse(raw: &str) -> Result<ScreenshotName, InvalidScreenshotName> {
        let name = parse_parts(raw).ok_or(InvalidScreenshotName)?;
        if name.to_string() == raw {
            Ok(name)
        } else {
            Err(InvalidScreenshotName)
        }
    }

    #[must_use]
    pub fn offset(&self) -> Duration {
        Duration::from_secs(self.offset_seconds)
    }
}

fn parse_parts(raw: &str) -> Option<ScreenshotName> {
    let stem = raw.strip_suffix(EXTENSION)?;
    let parts: Vec<&str> = stem.split(PART_SEPARATOR).collect();
    let (clock, suffix) = parts.split_at_checked(CLOCK_PARTS)?;
    let name = ScreenshotName::at(offset_from_clock(&clock.join(CLOCK_SEPARATOR))?);
    match suffix {
        [] => Some(name),
        [suffix] => parse_suffix(suffix).map(|suffix| ScreenshotName {
            suffix: Some(suffix),
            ..name
        }),
        _ => None,
    }
}

fn parse_suffix(raw: &str) -> Option<u16> {
    raw.parse()
        .ok()
        .filter(|suffix| (FIRST_SUFFIX..=LAST_SUFFIX).contains(suffix))
}

impl fmt::Display for ScreenshotName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let clock = offset_as_clock(self.offset()).replace(CLOCK_SEPARATOR, PART_SEPARATOR);
        match self.suffix {
            Some(suffix) => write!(formatter, "{clock}{PART_SEPARATOR}{suffix}{EXTENSION}"),
            None => write!(formatter, "{clock}{EXTENSION}"),
        }
    }
}
