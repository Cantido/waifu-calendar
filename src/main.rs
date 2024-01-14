use ics::{ICalendar, Event, properties::{DtStart, Summary}, parameters};
use uuid::Uuid;
use waifu_calendar::character::Character;

use anyhow::Result;
use clap::{Parser, Subcommand};
use time::{Duration, OffsetDateTime, Date};
use std::error::Error;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
  #[command(subcommand)]
  command: Option<Commands>
}

#[derive(Subcommand)]
enum Commands {
  Get {
    username: String,
  },
  Ics {
    username: String,
  }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
  let cli = Cli::parse();

  match &cli.command {
    Some(Commands::Get { username }) => {
      print_birthday_table(username.to_string()).await?;
    },
    Some(Commands::Ics { username }) => {
      let characters = waifu_calendar::get_waifu_birthdays(username.to_string()).await?;

      let mut calendar = ICalendar::new("2.0", "ics-rs");

      let now = OffsetDateTime::now_utc();

      for character in characters {
        let bd = character.birthday.next_occurrence(&now.date())?;

        let mut start = DtStart::new(date_to_dtstamp(bd));
        start.append(parameters!("VALUE" => "DATE"));

        let mut end = DtStart::new(date_to_dtstamp(bd + Duration::days(1)));
        end.append(parameters!("VALUE" => "DATE"));

        let mut event = Event::new(Uuid::now_v7().to_string(), datetime_to_dtstamp(now));

        event.push(Summary::new(format!("{}'s Birthday", character.name)));
        event.push(start);
        event.push(end);

        calendar.add_event(event);
      }

      calendar.save_file("birthdays.ics")?;

    },
    &None => {}
  }

  Ok(())
}

fn datetime_to_dtstamp(datetime: OffsetDateTime) -> String {
  format!("{:04}{:02}{:02}T{:02}{:02}{:02}", datetime.year(), datetime.month() as u8, datetime.day(), datetime.hour(), datetime.minute(), datetime.second())

}

fn date_to_dtstamp(date: Date) -> String {
  format!("{:04}{:02}{:02}", date.year(), date.month() as u8, date.day())

}

async fn print_birthday_table(username: String) -> Result<()> {
  println!("Fetching favorite character birthdays for username {}", username);

  let characters = waifu_calendar::get_waifu_birthdays(username).await?;
  let now = OffsetDateTime::now_utc();

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

  if !characters_bd_next_month.is_empty() {
    println!("\nUpcoming birthdays (next 30 days):\n");

    characters_bd_next_month.iter().for_each(|character| {
        println!("{}", character_row(character, &now));
    });
  }

  if !characters_bd_future.is_empty() {
    println!("\nFuture birthdays:\n");

    characters_bd_future.iter().for_each(|character| {
        println!("{}", character_row(character, &now));
    });
  }

  Ok(())
}

fn character_row(character: &Character, now: &OffsetDateTime) -> String {
  let til_next = character.birthday.til_next(*now);
  let next = character.birthday.next_occurrence(&now.date()).unwrap();
  let til_next_str = format!("{:.0}", til_next);
  format!("\t{:<20} {:>6} {:<15} {}", character.name, til_next_str, character.birthday.to_string(), next)
}


