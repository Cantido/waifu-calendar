//! Remember your favorite anime characters' birthdays.

#[cfg(feature = "http")]
pub mod http;

#[cfg(feature = "ics")]
pub mod ics;

use core::fmt;

use anyhow::{ensure, Context, Result, bail};
use graphql_client::{GraphQLQuery, Response};
use reqwest;
use serde::Serialize;
use time::{Date, Duration, Month, OffsetDateTime, Time};

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/schema.json",
    query_path = "src/birthdays.graphql",
    response_derives = "Debug"
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
        Self { month, day }
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
        let bd_date_this_year = self.to_date(today).with_context(|| {
            format!(
                "Failed to convert birthday into date with year {}",
                today.year()
            )
        })?;

        let (_today_year, today_ordinal) = today.to_ordinal_date();
        let (_today_year, bd_ordinal) = bd_date_this_year.to_ordinal_date();

        let next = if today_ordinal <= bd_ordinal {
            // Birthday hasn't happened yet this year
            bd_date_this_year
        } else {
            // Birthday has already happened this year
            self.to_date(today).with_context(|| {
                format!(
                    "Failed to convert birthday into date with year {}",
                    today.year() + 1
                )
            })?
        };

        ensure!(
            today <= &next,
            "Somehow came up with a next occurrence that wasn't after now"
        );

        Ok(next)
    }

    /// Returns the first occurrence of the birthday that is strictly later than a given `Date`.
    pub fn to_date(&self, today: &Date) -> Result<Date> {
        let current_year = today.year();
        let occurrence_year =
            if self.month() == Month::February && self.day() == 29 {
                let til_leap_year = 4 - (current_year % 4);
                today.year() + til_leap_year
            } else {
                today.year()
            };
        let date = Date::from_calendar_date(occurrence_year, self.month, self.day).with_context(|| {
            format!(
                "Failed to build date from birthday {:?} in year {}",
                self, occurrence_year
            )
        })?;

        Ok(date)
    }

    /// Calculate the `Duration` between now and this birthday.
    pub fn til_next(&self, now: &OffsetDateTime) -> Duration {
        let next_date = self.next_occurrence(&now.date()).unwrap();
        let next = OffsetDateTime::new_in_offset(next_date, Time::MIDNIGHT, now.offset());

        next - *now
    }

    pub fn to_iso_string(&self) -> String {
        format!("{:02}-{:02}", self.month as u8, self.day)
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
    url: String,
    birthday: Birthday,
}

impl Character {
    /// Create a new Character.
    pub fn new(name: &str, url: &str, birthday: Birthday) -> Self {
        Self {
            name: name.to_string(),
            url: url.to_string(),
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
        let (characters_bd_today, characters_bd_future): (Vec<Character>, Vec<Character>) = self
            .into_iter()
            .partition(|character| character.birthday().is_occurring_on(&now.date()));

        let in_thirty_days = *now + Duration::days(30);

        let (characters_bd_next_month, characters_bd_future): (Vec<Character>, Vec<Character>) =
            characters_bd_future.into_iter().partition(|character| {
                let next = character.birthday().next_occurrence(&now.date()).unwrap();
                next <= in_thirty_days.date()
            });

        BirthdayCategories {
            today: characters_bd_today,
            within_thirty_days: characters_bd_next_month,
            future: characters_bd_future,
        }
    }
}

#[derive(thiserror::Error, Debug, Clone)]
pub enum Error {
    #[error("user name {0} not found")]
    UserNotFound(String),
    #[error("received an unexpected response from AniList")]
    BadResponse,
    #[error("Rate limited by the AniList API")]
    RateLimited,
}

/// Get the favorite character birthdays for an AniList user.
///
/// Characters are not sorted.
/// See the `Characters` trait for sort options.
/// Uses AniList's GraphQL API to fetch data on favorites.
pub async fn get_waifu_birthdays(username: &str) -> Result<Vec<Character>> {
    let mut page = 1;
    let mut has_next_page = true;

    let mut characters = vec![];

    while has_next_page {
        let variables = birthdays_query::Variables {
            page,
            user: username.to_string(),
        };

        let request_body = BirthdaysQuery::build_query(variables);

        let client = reqwest::Client::new();
        let res = client
            .post("https://graphql.anilist.co")
            .header("User-Agent", "WaifuCalendar")
            .json(&request_body)
            .send()
            .await?;

        if res.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            bail!(Error::RateLimited);
        }

        let response_body: Response<birthdays_query::ResponseData> = res.json().await?;

        let data = response_body
            .data
            .ok_or(Error::BadResponse)
            .with_context(|| "Missing response data")?;

        let response_page = data
            .user
            .ok_or(Error::UserNotFound(username.to_string()))?
            .favourites
            .ok_or(Error::BadResponse)
            .with_context(|| "Missing favourites")?
            .characters
            .ok_or(Error::BadResponse)
            .with_context(|| "Missing characters")?;

        let mut page_characters: Vec<Character> = response_page
            .nodes
            .ok_or(Error::BadResponse)
            .with_context(|| "Missing character nodes")?
            .iter()
            .filter_map(|node_result| {
                let node = node_result.as_ref()?;
                let dob = node.date_of_birth.as_ref()?;

                let month_opt = dob.month.as_ref();
                let day_opt = dob.day.as_ref();

                if month_opt.is_some() && day_opt.is_some() {
                    let month_num: u8 = month_opt?.to_owned().try_into().ok()?;
                    let month = Month::try_from(month_num).ok()?;
                    let day: u8 = day_opt?.to_owned().try_into().ok()?;

                    let birthday = Birthday::new(month, day);

                    let name = node.name.as_ref()?.full.as_ref()?.to_string();

                    let url = node.site_url.as_ref()?.to_string();

                    let character = Character { name, url, birthday };

                    Some(character)
                } else {
                    None
                }
            })
            .collect();

            characters.append(&mut page_characters);

            has_next_page =
                response_page
                .page_info.ok_or(Error::BadResponse).with_context(|| "Missing page_info")?
                .has_next_page.ok_or(Error::BadResponse).with_context(|| "Missing has_next_page")?;

            page += 1;
        }

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
        let date = bd.to_date(&Date::from_calendar_date(2024, Month::January, 1).unwrap()).unwrap();

        assert_eq!(date.year(), 2024);
        assert_eq!(date.month(), bd.month);
        assert_eq!(date.day(), bd.day);
    }

    #[test]
    fn to_date_leap_year() {
        let bd = Birthday::new(Month::February, 29);
        let date = bd.to_date(&Date::from_calendar_date(2025, Month::January, 1).unwrap()).unwrap();

        assert_eq!(date.year(), 2028);
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
