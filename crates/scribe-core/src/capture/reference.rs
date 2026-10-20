use std::fmt;

use crate::capture::ScreenshotName;
use crate::recording::SessionId;

const SEPARATOR: char = '/';

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("not a valid screenshot reference")]
pub struct InvalidScreenshotReference;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScreenshotReference {
    session: SessionId,
    name: ScreenshotName,
}

impl ScreenshotReference {
    #[must_use]
    pub fn new(session: SessionId, name: ScreenshotName) -> ScreenshotReference {
        ScreenshotReference { session, name }
    }

    pub fn parse(raw: &str) -> Result<ScreenshotReference, InvalidScreenshotReference> {
        let (session, name) = raw
            .split_once(SEPARATOR)
            .ok_or(InvalidScreenshotReference)?;
        Ok(ScreenshotReference::new(
            SessionId::parse(session).map_err(|_| InvalidScreenshotReference)?,
            ScreenshotName::parse(name).map_err(|_| InvalidScreenshotReference)?,
        ))
    }

    #[must_use]
    pub fn session(&self) -> &SessionId {
        &self.session
    }

    #[must_use]
    pub fn name(&self) -> &ScreenshotName {
        &self.name
    }
}

impl fmt::Display for ScreenshotReference {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}{SEPARATOR}{}", self.session, self.name)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServedImage {
    pub png: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub original_width: u32,
    pub original_height: u32,
}

impl ServedImage {
    #[must_use]
    pub fn was_scaled(&self) -> bool {
        (self.width, self.height) != (self.original_width, self.original_height)
    }
}
