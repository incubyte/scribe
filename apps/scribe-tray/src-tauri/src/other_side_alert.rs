pub struct Alert {
    pub title: &'static str,
    pub body: &'static str,
}

pub const OTHER_SIDE_UNAVAILABLE_ON_WINDOWS: Alert = Alert {
    title: "Scribe can't capture the other side of the call",
    body: "Only your microphone is being recorded. Check that an output device such as speakers or headphones is connected and enabled, then start a new recording.",
};
