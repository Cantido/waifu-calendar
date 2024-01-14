pub mod ics;

use core::fmt;

use anyhow::{ensure, Result};
use graphql_client::{GraphQLQuery, Response};
use time::{Month, OffsetDateTime, Date, Time, Duration};
use reqwest;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/schema.json",
    query_path = "src/birthdays.graphql",
    response_derives = "Debug",
)]
struct BirthdaysQuery;

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

  pub fn from_date(date: &Date) -> Self {
    Self {
      month: date.month(),
      day: date.day(),
    }
  }

  pub fn is_occurring_on(&self, date: &Date) -> bool {
    self.month == date.month() && self.day == date.day()
  }

  pub fn next_occurrence(&self, today: &Date) -> Result<Date> {
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

    ensure!(today <= &next, "Somehow came up with a next occurrence that wasn't after now");

    Ok(next)
  }

  pub fn to_date(&self, year: i32) -> Result<Date> {
    Ok(Date::from_calendar_date(year, self.month, self.day)?)
  }

  pub fn til_next(&self, now: &OffsetDateTime) -> Duration {
    let next = OffsetDateTime::new_utc(self.next_occurrence(&now.date()).unwrap(), Time::MIDNIGHT);

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

pub async fn get_waifu_birthdays(username: &str, now: &OffsetDateTime) -> Result<Vec<Character>> {
  let variables = birthdays_query::Variables {
    user: username.to_string(),
  };

  let request_body = BirthdaysQuery::build_query(variables);

  let client = reqwest::Client::new();
  let res = client.post("https://graphql.anilist.co").json(&request_body).send().await?;
  let response_body: Response<birthdays_query::ResponseData> = res.json().await?;

  let data = response_body.data.expect("Missing response data");

  let mut characters: Vec<Character> =
    data.user.expect("Missing user")
        .favourites.expect("Missing favourites")
        .characters.expect("Missing characters")
        .nodes.expect("Missing character nodes")
        .iter().filter_map(|node_result| {
          let node = node_result.as_ref().unwrap();
          let dob = node.date_of_birth.as_ref().expect("Missing character date of birth");

          let month_opt = dob.month.as_ref();
          let day_opt = dob.day.as_ref();

          if month_opt.is_some() && day_opt.is_some() {
            let month_num: u8 = month_opt.unwrap().to_owned().try_into().unwrap();
            let month = Month::try_from(month_num).unwrap();
            let day: u8 = day_opt.unwrap().to_owned().try_into().unwrap();

            let birthday = Birthday::new(month, day);

            let name = node.name.as_ref().unwrap().full.as_ref().unwrap().to_string();

            let character = Character {
              name,
              birthday,
            };

            Some(character)
          } else {
            None
          }
        }).collect();

  characters.sort_by(|a, b| {
    let til_a = a.birthday.til_next(now);
    let til_b = b.birthday.til_next(now);

    til_a.cmp(&til_b)
  });

  Ok(characters)
}

#[cfg(test)]
mod tests {
    use time::{Date, Month};

    use crate::Birthday;

  #[test]
  fn next_occurrence_is_today() {
    let bd = Birthday::new(Month::January, 13);
    let today = Date::from_calendar_date(2024, Month::January, 13).unwrap();

    let next = bd.next_occurrence(&today).unwrap();

    assert_eq!(next, today);
  }

  #[test]
  fn next_occurrence_is_this_year() {
    let bd = Birthday::new(Month::January, 15);
    let today = Date::from_calendar_date(2024, Month::January, 13).unwrap();

    let next = bd.next_occurrence(&today).unwrap();

    assert_eq!(next.year(), 2024);
    assert_eq!(next.month(), bd.month);
    assert_eq!(next.day(), bd.day);
  }

  #[test]
  fn next_occurrence_is_next_year() {
    let bd = Birthday::new(Month::January, 1);
    let today = Date::from_calendar_date(2024, Month::January, 13).unwrap();

    let next = bd.next_occurrence(&today).unwrap();

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
    let bd = Birthday::from_date(&date);

    assert_eq!(bd.month, Month::January);
    assert_eq!(bd.day, 13);
  }

  #[test]
  fn is_occurring_on_same_date() {
    let date = Date::from_calendar_date(2024, Month::January, 13).unwrap();
    let bd = Birthday::new(Month::January, 13);

    assert!(bd.is_occurring_on(&date));
  }

  #[test]
  fn is_occurring_on_different_date() {
    let date = Date::from_calendar_date(2024, Month::January, 14).unwrap();
    let bd = Birthday::new(Month::January, 13);

    assert!(!bd.is_occurring_on(&date));
  }
}
