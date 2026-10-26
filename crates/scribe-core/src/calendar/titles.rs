use super::meeting::Meeting;

#[must_use]
pub fn meeting_named_in_titles<'a>(
    candidates: &[&'a Meeting],
    titles: &[String],
) -> Option<&'a Meeting> {
    let normalized_titles: Vec<String> = titles.iter().map(|title| normalized(title)).collect();
    let mut named = candidates
        .iter()
        .copied()
        .filter(|meeting| is_named_in(meeting, &normalized_titles));
    match (named.next(), named.next()) {
        (Some(only), None) => Some(only),
        _ => None,
    }
}

fn is_named_in(meeting: &Meeting, normalized_titles: &[String]) -> bool {
    let subject = normalized(&meeting.subject);
    !subject.is_empty()
        && normalized_titles
            .iter()
            .any(|title| title.contains(&subject))
}

fn normalized(text: &str) -> String {
    text.trim().to_lowercase()
}
