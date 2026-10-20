use scribe_core::capture::{ScreenshotReference, ServedImage};
use scribe_core::recording::offset_as_clock;

pub const MAX_SERVED_EDGE_PX: u32 = 1568;

const SECTION_INTRO: &str = "Screenshots taken during this call. Each name is its offset into the call and matches the transcript's [HH:MM:SS] stamps. Call get_screenshot with a name to see it.";

#[must_use]
pub fn screenshots_section(references: &[ScreenshotReference]) -> Option<String> {
    if references.is_empty() {
        return None;
    }
    let lines: Vec<String> = references
        .iter()
        .map(|reference| format!("- {}", stamped(reference)))
        .collect();
    Some(format!(
        "\n## Screenshots\n\n{SECTION_INTRO}\n\n{}\n",
        lines.join("\n")
    ))
}

#[must_use]
pub fn served_label(reference: &ScreenshotReference, image: &ServedImage) -> String {
    let size = format!("{} × {}", image.width, image.height);
    if image.was_scaled() {
        format!(
            "{} — {size} (scaled from {} × {})",
            stamped(reference),
            image.original_width,
            image.original_height
        )
    } else {
        format!("{} — {size}", stamped(reference))
    }
}

fn stamped(reference: &ScreenshotReference) -> String {
    format!(
        "[{}] {reference}",
        offset_as_clock(reference.name().offset())
    )
}
