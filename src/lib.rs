//! Remember your favorite anime characters' birthdays.

pub mod http;
pub mod ics;

use core::fmt;

use anyhow::{Context, ensure, Result};
use graphql_client::{GraphQLQuery, Response};
use serde::Serialize;
use time::{Month, OffsetDateTime, Date, Time, Duration};
use reqwest;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/schema.json",
    query_path = "src/birthdays.graphql",
    response_derives = "Debug",
)]
struct BirthdaysQuery;

/// A `Month` and day pair.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug, Serialize)]
pub struct Birthday {
  month: Month,
  day: u8,
}

impl Birthday {
  /// Build a new `Birthday` that occurs on the given month and day.
  pub fn new(month: Month, day: u8) -> Self {
    Self {
      month,
      day,
    }
  }

  /// Get the month this birthday occurs in.
  pub fn month(&self) -> Month {
    self.month
  }

  /// Get the day of the month this birthday occurs on.
  pub fn day(&self) -> u8 {
    self.day
  }

  /// Build a new `Birthday` that occurred on a `Date`.
  pub fn from_date(date: &Date) -> Self {
    Self {
      month: date.month(),
      day: date.day(),
    }
  }

  /// Check if this birthday will occur on the given `Date`.
  pub fn is_occurring_on(&self, date: &Date) -> bool {
    self.month == date.month() && self.day == date.day()
  }

  /// Get the next `Date` that this birthday will occur on.
  pub fn next_occurrence(&self, today: &Date) -> Result<Date> {
    let bd_date_this_year = self.to_date(today.year())
      .with_context(|| format!("Failed to convert birthday into date with year {}", today.year()))?;

    let (_today_year, today_ordinal) = today.to_ordinal_date();
    let (_today_year, bd_ordinal) = bd_date_this_year.to_ordinal_date();

    let next =
      if today_ordinal <= bd_ordinal {
        // Birthday hasn't happened yet this year
        bd_date_this_year
      } else {
        // Birthday has already happened this year
        self.to_date(today.year() + 1)
          .with_context(|| format!("Failed to convert birthday into date with year {}", today.year() + 1))?
      };

    ensure!(today <= &next, "Somehow came up with a next occurrence that wasn't after now");

    Ok(next)
  }

  /// Returns a `Date` with the month & day of this birthday, occurring in the given year.
  pub fn to_date(&self, year: i32) -> Result<Date> {
    let date =
      Date::from_calendar_date(year, self.month, self.day)
        .with_context(|| format!("Failed to build date from birthday {:?} in year {}", self, year))?;

    Ok(date)
  }

  /// Calculate the `Duration` between now and this birthday.
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

/// A name and birthday pair.
#[derive(Clone, Eq, PartialEq, Hash, Debug, Serialize)]
pub struct Character {
  name: String,
  birthday: Birthday,
}

impl Character {
  /// Create a new Character.
  pub fn new(name: &str, birthday: Birthday) -> Self {
    Self {
      name: name.to_string(),
      birthday,
    }
  }

  /// Get this character's name.
  pub fn name(&self) -> &str {
    &self.name
  }

  /// Get this character's birthday
  pub fn birthday(&self) -> Birthday {
    self.birthday
  }
}

#[derive(Clone, Eq, PartialEq, Hash, Debug, Serialize)]
pub struct BirthdayCategories {
  pub today: Vec<Character>,
  pub within_thirty_days: Vec<Character>,
  pub future: Vec<Character>,
}

/// Useful functions for working with a collection of characters.
///
/// # Examples
///
/// ```
/// use time::OffsetDateTime;
///
/// // Sorts the characters so the closest upcoming birthdays are first.
/// let characters = waifu_calendar::get_waifu_birthdays("cosmicrose");
/// characters.sort_by_upcoming(OffsetDateTime::now_utc());
/// ```
pub trait Characters {
  fn sort_by_upcoming(&mut self, now: &OffsetDateTime);
  fn into_birthday_categories(self, now: &OffsetDateTime) -> BirthdayCategories;
}

impl Characters for Vec<Character> {
  fn sort_by_upcoming(&mut self, now: &OffsetDateTime) {
    self.sort_by(|a, b| {
      let til_a = a.birthday().til_next(now);
      let til_b = b.birthday().til_next(now);

      til_a.cmp(&til_b)
    });
  }

  fn into_birthday_categories(self, now: &OffsetDateTime) -> BirthdayCategories {
    let (characters_bd_today, characters_bd_future): (Vec<Character>, Vec<Character>) = self.into_iter().partition(|character| {
      character.birthday().is_occurring_on(&now.date())
    });

    let in_thirty_days = *now + Duration::days(30);

    let (characters_bd_next_month, characters_bd_future): (Vec<Character>, Vec<Character>) = characters_bd_future.into_iter().partition(|character| {
      let next = character.birthday().next_occurrence(&now.date()).unwrap();
      next <= in_thirty_days.date()
    });

    BirthdayCategories {
      today: characters_bd_today,
      within_thirty_days: characters_bd_next_month,
      future: characters_bd_future
    }
  }
}

/// Get the favorite character birthdays for an AniList user.
///
/// Characters are not sorted.
/// See the `Characters` trait for sort options.
/// Uses AniList's GraphQL API to fetch data on favorites.
///
/// # Examples
///
/// ```
/// use time::OffsetDateTime;
///
/// // Prints every character's name and birthday
/// let now = OffsetDateTime::now_utc();
/// for character in waifu_calendar::get_waifu_birthdays("cosmicrose", &now).iter() {
///   println!("{}: {}", character.name(), character.birthday());
/// }
/// ```
pub async fn get_waifu_birthdays(username: &str) -> Result<Vec<Character>> {
  let variables = birthdays_query::Variables {
    user: username.to_string(),
  };

  let request_body = BirthdaysQuery::build_query(variables);

  let client = reqwest::Client::new();
  let res = client.post("https://graphql.anilist.co").json(&request_body).send().await?;
  let response_body: Response<birthdays_query::ResponseData> = res.json().await?;

  let data = response_body.data.expect("Missing response data");

  let characters: Vec<Character> =
    data.user.with_context(|| "Missing user")?
        .favourites.with_context(|| "Missing favourites")?
        .characters.with_context(|| "Missing characters")?
        .nodes.with_context(|| "Missing character nodes")?
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
