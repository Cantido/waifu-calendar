//! Tools for making ICalendar data.

use ics::{ICalendar, Event, properties::{DtStart, Summary}, parameters};
use uuid::Uuid;
use crate::Character;

use anyhow::Result;
use time::{Duration, OffsetDateTime, Date};

/// Convert character birthdays into ICalendar format.
pub trait BirthdayICalendar {
  /// Returns an ICalendar-formatted string.
  fn to_ics(&self, now: &OffsetDateTime) -> Result<String>;
}

impl BirthdayICalendar for Vec<Character> {
  fn to_ics(&self, now: &OffsetDateTime) -> Result<String> {
    let mut calendar = ICalendar::new("2.0", "ics-rs");

    for character in self {
      let bd = character.birthday().next_occurrence(&now.date())?;

      let mut start = DtStart::new(date_to_dtstamp(&bd));
      start.append(parameters!("VALUE" => "DATE"));

      let mut end = DtStart::new(date_to_dtstamp(&(bd + Duration::days(1))));
      end.append(parameters!("VALUE" => "DATE"));

      let mut event = Event::new(Uuid::now_v7().to_string(), datetime_to_dtstamp(now));

      event.push(Summary::new(format!("{}'s Birthday", character.name())));
      event.push(start);
      event.push(end);

      calendar.add_event(event);
    }

    Ok(calendar.to_string())
  }
}


fn datetime_to_dtstamp(datetime: &OffsetDateTime) -> String {
  format!("{:04}{:02}{:02}T{:02}{:02}{:02}", datetime.year(), datetime.month() as u8, datetime.day(), datetime.hour(), datetime.minute(), datetime.second())

}

fn date_to_dtstamp(date: &Date) -> String {
  format!("{:04}{:02}{:02}", date.year(), date.month() as u8, date.day())

}
