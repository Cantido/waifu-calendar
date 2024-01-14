use waifu_calendar::{Character, Characters, ics::BirthdayICalendar};

use anyhow::{Result, Context};
use clap::{Parser, Subcommand};
use shadow_rs::shadow;
use time::{Duration, OffsetDateTime};
use std::{error::Error, path::PathBuf, fs::File, io::Write, env::current_dir};

shadow!(build);

#[derive(Debug, Parser)]
#[command(author, version = build::CLAP_LONG_VERSION, about, long_about = None)]
struct Cli {
  #[command(subcommand)]
  command: Option<Commands>
}

#[derive(Debug, Subcommand)]
enum Commands {
  /// Output birthdays to stdout
  Get {
    /// The AniList user to fetch favorite characters from
    username: String,
  },
  /// Output birthdays to ICalendar (*.ics) format
  Ics {
    /// The AniList user to fetch favorite characters from
    username: String,

    /// Output ICalendar to a file instead of to stdout
    #[arg(short, long, value_name = "FILE")]
    output: Option<PathBuf>,
  }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
  let cli = Cli::parse();

  match &cli.command {
    Some(Commands::Get { username }) => {
      let now = OffsetDateTime::now_utc();
      print_birthday_table(username, &now).await?;
    },
    Some(Commands::Ics { username, output }) => {
      let cal = {
        let now = OffsetDateTime::now_utc();
        let mut characters = waifu_calendar::get_waifu_birthdays(username).await
            .with_context(|| format!("Failed to get waifu birthdays for user {}", username))?;
        characters.sort_by_upcoming(&now);
        characters.to_ics(&now)
            .with_context(|| "Failed to convert character collection into ics")?
      };

      if let Some(path) = output {
        let path =
          if path.is_absolute() {
            path.to_owned()
          } else {
            let cwd = current_dir()
              .with_context(|| "Failed to get current working dir")?;
            cwd.join(path)
          };

        let mut file = File::options()
            .read(true)
            .write(true)
            .create(true)
            .open(&path)
            .with_context(|| format!("Failed to open output ICS file at {:?}", &path))?;

        file.write_all(&cal.as_bytes())
          .with_context(|| "Failed to write ICS to given output file")?;
      } else {
        println!("{}", cal);
      }
    },
    &None => {}
  }

  Ok(())
}

async fn print_birthday_table(username: &str, now: &OffsetDateTime) -> Result<()> {
  println!("Fetching favorite character birthdays for username {}", username);

  let characters = {
    let mut characters = waifu_calendar::get_waifu_birthdays(username).await
      .with_context(|| format!("Failed to get waifu birthdays for user {}", username))?;
    characters.sort_by_upcoming(&now);
    characters
  };

  let (characters_bd_today, characters_bd_future): (Vec<Character>, Vec<Character>) = characters.into_iter().partition(|character| {
    character.birthday().is_occurring_on(&now.date())
  });

  if !characters_bd_today.is_empty() {
    println!("Birthdays TODAY ({}):\n", now.date());

    characters_bd_today.iter().for_each(|character| {
      println!("\t{}", character.name());
    });
  }

  let in_thirty_days = *now + Duration::days(30);

  let (characters_bd_next_month, characters_bd_future): (Vec<Character>, Vec<Character>) = characters_bd_future.into_iter().partition(|character| {
    let next = character.birthday().next_occurrence(&now.date()).unwrap();
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
  let til_next = character.birthday().til_next(now);
  let next = character.birthday().next_occurrence(&now.date()).unwrap();
  let til_next_str = format!("{:.0}", til_next);
  format!("\t{:<20} {:>6} {:<15} {}", character.name(), til_next_str, character.birthday().to_string(), next)
}


