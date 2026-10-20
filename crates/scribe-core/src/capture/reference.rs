use std::fmt;

use crate::capture::ScreenshotName;
use crate::recording::SessionId;

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
        write!(formatter, "{}/{}", self.session, self.name)
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
