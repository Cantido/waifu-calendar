use core::fmt;

use anyhow::{Result, ensure};
use time::{Month, OffsetDateTime, Date, Time, Duration};

pub struct Birthday {
  pub month: Month,
  pub day: u8,
}

impl Birthday {
  pub fn new(month: Month, day: u8) -> Self {
    Self {
      month,
      day,
    }
  }

  pub fn from_date(date: Date) -> Self {
    Self {
      month: date.month(),
      day: date.day(),
    }
  }

  pub fn is_occurring_on(&self, date: Date) -> bool {
    self.month == date.month() && self.day == date.day()
  }

  pub fn next_occurrence(&self, today: Date) -> Result<Date> {
    let (today_year, today_ordinal) = today.to_ordinal_date();
    let (_today_year, bd_ordinal) = self.to_date(today_year)?.to_ordinal_date();

    let next =
      if today_ordinal <= bd_ordinal {
        // Birthday hasn't happened yet this year
        Date::from_calendar_date(today.year(), self.month, self.day)?
      } else {
        // Birthday has already happened this year
        Date::from_calendar_date(today.year() + 1, self.month, self.day)?
      };

    ensure!(today <= next, "Somehow came up with a next occurrence that wasn't after now");

    Ok(next)
  }

  pub fn to_date(&self, year: i32) -> Result<Date> {
    Ok(Date::from_calendar_date(year, self.month, self.day)?)
  }

  pub fn til_next(&self, now: OffsetDateTime) -> Duration {
    let next = OffsetDateTime::new_utc(self.next_occurrence(now.date()).unwrap(), Time::MIDNIGHT);

    let duration_seconds = (next.unix_timestamp() - now.unix_timestamp()).try_into().unwrap();

    Duration::new(duration_seconds, 0)
  }
}

impl fmt::Display for Birthday {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
      write!(f, "{} {}", self.month, self.day)
  }
}

pub struct Character {
  pub name: String,
  pub birthday: Birthday,
}

#[cfg(test)]
mod tests {
    use time::{Date, Month};

    use crate::Birthday;

  #[test]
  fn next_occurrence_is_today() {
    let bd = Birthday::new(Month::January, 13);
    let today = Date::from_calendar_date(2024, Month::January, 13).unwrap();

    let next = bd.next_occurrence(today).unwrap();

    assert_eq!(next, today);
  }

  #[test]
  fn next_occurrence_is_this_year() {
    let bd = Birthday::new(Month::January, 15);
    let today = Date::from_calendar_date(2024, Month::January, 13).unwrap();

    let next = bd.next_occurrence(today).unwrap();

    assert_eq!(next.year(), 2024);
    assert_eq!(next.month(), bd.month);
    assert_eq!(next.day(), bd.day);
  }

  #[test]
  fn next_occurrence_is_next_year() {
    let bd = Birthday::new(Month::January, 1);
    let today = Date::from_calendar_date(2024, Month::January, 13).unwrap();

    let next = bd.next_occurrence(today).unwrap();

    assert_eq!(next.year(), 2025);
    assert_eq!(next.month(), bd.month);
    assert_eq!(next.day(), bd.day);
  }

  #[test]
  fn to_date() {
    let bd = Birthday::new(Month::January, 13);
    let date = bd.to_date(2024).unwrap();

    assert_eq!(date.year(), 2024);
    assert_eq!(date.month(), bd.month);
    assert_eq!(date.day(), bd.day);
  }

  #[test]
  fn from_date() {
    let date = Date::from_calendar_date(2024, Month::January, 13).unwrap();
    let bd = Birthday::from_date(date);

    assert_eq!(bd.month, Month::January);
    assert_eq!(bd.day, 13);
  }

  #[test]
  fn is_occurring_on_same_date() {
    let date = Date::from_calendar_date(2024, Month::January, 13).unwrap();
    let bd = Birthday::new(Month::January, 13);

    assert!(bd.is_occurring_on(date));
  }

  #[test]
  fn is_occurring_on_different_date() {
    let date = Date::from_calendar_date(2024, Month::January, 14).unwrap();
    let bd = Birthday::new(Month::January, 13);

    assert!(!bd.is_occurring_on(date));
  }
}
