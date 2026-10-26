use std::sync::{Mutex, MutexGuard, PoisonError};

use chrono::NaiveDate;
use scribe_core::calendar::{
    ArmedAction, ArmedEntry, ArmedMeetings, ArmedState, ArmedWatch, EventId, WatchInputs,
};
use serde_json::{Value, json};

use super::agenda_view::{AgendaView, Arming};
use super::selection::AutoStart;

pub const ARMED_MEETINGS_KEY: &str = "armed_meetings";
const EVENT_ID_FIELD: &str = "event_id";
const DATE_FIELD: &str = "date";
const DATE_FORMAT: &str = "%Y-%m-%d";

#[must_use]
pub fn entries_from(saved: Option<&Value>) -> Vec<ArmedEntry> {
    saved
        .and_then(Value::as_array)
        .map(|entries| entries.iter().filter_map(entry_from).collect())
        .unwrap_or_default()
}

fn entry_from(saved: &Value) -> Option<ArmedEntry> {
    let event_id = saved.get(EVENT_ID_FIELD)?.as_str()?;
    let date = saved.get(DATE_FIELD)?.as_str()?;
    Some(ArmedEntry {
        event_id: EventId::new(event_id),
        date: NaiveDate::parse_from_str(date, DATE_FORMAT).ok()?,
    })
}

#[must_use]
pub fn settings_value(armed: &ArmedMeetings) -> Value {
    armed
        .entries()
        .iter()
        .map(|entry| {
            json!({
                EVENT_ID_FIELD: entry.event_id.as_str(),
                DATE_FIELD: entry.date.format(DATE_FORMAT).to_string(),
            })
        })
        .collect()
}

pub struct ArmedStore {
    armed: Mutex<ArmedMeetings>,
    watch: Mutex<ArmedWatch>,
    interrupted: Mutex<Option<EventId>>,
}

impl ArmedStore {
    #[must_use]
    pub fn new(armed: ArmedMeetings) -> ArmedStore {
        ArmedStore {
            armed: Mutex::new(armed),
            watch: Mutex::new(ArmedWatch::new()),
            interrupted: Mutex::new(None),
        }
    }

    pub fn prune_for(&self, today: NaiveDate) -> bool {
        let mut armed = locked(&self.armed);
        let pruned = ArmedMeetings::for_day(armed.entries().to_vec(), today);
        let changed = pruned != *armed;
        *armed = pruned;
        changed
    }

    #[must_use]
    pub fn armed(&self) -> ArmedMeetings {
        locked(&self.armed).clone()
    }

    pub fn arm(&self, id: EventId, today: NaiveDate) {
        locked(&self.armed).arm(id, today);
    }

    pub fn disarm(&self, id: &EventId) {
        locked(&self.armed).disarm(id);
        locked(&self.watch).forget(id);
    }

    #[must_use]
    pub fn arming_of(&self, id: &EventId) -> Arming {
        if !locked(&self.armed).is_armed(id) {
            return Arming::Off;
        }
        match locked(&self.watch).state_of(id) {
            ArmedState::Missed(_) => Arming::Missed,
            ArmedState::Armed | ArmedState::Recording | ArmedState::Recorded => Arming::Armed,
        }
    }

    #[must_use]
    pub fn mark(&self, view: AgendaView) -> AgendaView {
        view.marked(|id| self.arming_of(id))
    }

    pub fn take_due_actions(&self, inputs: &WatchInputs) -> Vec<ArmedAction> {
        locked(&self.watch).take_due_actions(inputs)
    }

    #[must_use]
    pub fn start_for(&self, id: &EventId) -> AutoStart {
        if locked(&self.armed).is_armed(id) {
            AutoStart::Yes
        } else {
            AutoStart::No
        }
    }

    pub fn note_recording(&self, id: &EventId) {
        locked(&self.watch).note_recording(id);
    }

    pub fn note_recorded(&self, id: &EventId) {
        locked(&self.watch).note_recorded(id);
    }

    pub fn note_start_failed(&self, id: &EventId) {
        locked(&self.watch).note_start_failed(id);
    }

    pub fn remember_interrupted(&self, id: EventId) {
        *locked(&self.interrupted) = Some(id);
    }

    pub fn resume_interrupted(&self) {
        if let Some(id) = locked(&self.interrupted).take() {
            locked(&self.watch).forget(&id);
        }
    }
}

fn locked<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}
