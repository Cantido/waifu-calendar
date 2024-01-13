use std::env;

use anyhow::Result;
use time::OffsetDateTime;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
  let username = env::args().collect::<Vec<String>>()[1].to_string();

  println!("Fetching favorite character birthdays for username {}", username);

  let now = OffsetDateTime::now_utc();
  let characters = waifu_calendar::get_waifu_birthdays(username).await?;

  characters.iter().for_each(|character| {
    if character.birthday.is_occurring_on(now.date()) {
      println!("{}: TODAY! ({})", character.name, now.date());
    } else {
      let til_next = character.birthday.til_next(now);
      let next = character.birthday.next_occurrence(now.date()).unwrap();

      println!("{}: {} (next: in {:.0} on {})", character.name, character.birthday, til_next, next);
    }
  });

  Ok(())
}

