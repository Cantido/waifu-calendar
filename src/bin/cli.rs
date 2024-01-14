use std::env;

use waifu_calendar::character::Character;

use anyhow::Result;
use time::{Duration, OffsetDateTime};
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
  let username = env::args().collect::<Vec<String>>()[1].to_string();

  println!("Fetching favorite character birthdays for username {}", username);

  let now = OffsetDateTime::now_utc();
  let characters = waifu_calendar::get_waifu_birthdays(username).await?;

  let (characters_bd_today, characters_bd_future): (Vec<Character>, Vec<Character>) = characters.into_iter().partition(|character| {
    character.birthday.is_occurring_on(&now.date())
  });

  if !characters_bd_today.is_empty() {
    println!("Birthdays TODAY ({}):\n", now.date());

    characters_bd_today.iter().for_each(|character| {
      println!("\t{}", character.name);
    });
  }

  let in_thirty_days = now + Duration::days(30);

  let (characters_bd_next_month, characters_bd_future): (Vec<Character>, Vec<Character>) = characters_bd_future.into_iter().partition(|character| {
    let next = character.birthday.next_occurrence(&now.date()).unwrap();
    next <= in_thirty_days.date()
  });

  println!("\nUpcoming birthdays (next 30 days):\n");

  characters_bd_next_month.iter().for_each(|character| {
      println!("{}", character_row(character, &now));
  });

  println!("\nFuture birthdays:\n");

  characters_bd_future.iter().for_each(|character| {
      println!("{}", character_row(character, &now));
  });

  Ok(())
}

fn character_row(character: &Character, now: &OffsetDateTime) -> String {
  let til_next = character.birthday.til_next(*now);
  let next = character.birthday.next_occurrence(&now.date()).unwrap();
  let til_next_str = format!("{:.0}", til_next);
  format!("\t{:<20} {:>6} {:<15} {}", character.name, til_next_str, character.birthday.to_string(), next)
}


