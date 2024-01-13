pub mod character;

use anyhow::Result;
use graphql_client::{GraphQLQuery, Response};
use time::{Month, OffsetDateTime};
use reqwest;

use crate::character::{Birthday, Character};

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/schema.json",
    query_path = "src/birthdays.graphql",
    response_derives = "Debug",
)]
struct BirthdaysQuery;

pub async fn get_waifu_birthdays(username: String) -> Result<Vec<Character>> {
  let variables = birthdays_query::Variables {
    user: username,
  };

  let request_body = BirthdaysQuery::build_query(variables);

  let client = reqwest::Client::new();
  let res = client.post("https://graphql.anilist.co").json(&request_body).send().await?;
  let response_body: Response<birthdays_query::ResponseData> = res.json().await?;

  let data = response_body.data.expect("Missing response data");

  let now = OffsetDateTime::now_utc();

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
