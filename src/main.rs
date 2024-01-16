use log::info;
use waifu_calendar::{Character, Characters, ics::BirthdayICalendar};

use anyhow::{Result, Context};
use clap::{Parser, Subcommand};
use shadow_rs::shadow;
use time::OffsetDateTime;
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
  },
  Serve,
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
    Some(Commands::Serve) => {
      env_logger::init();

      let bind_addr = "0.0.0.0:8080";

      info!("starting Waifu Calendar on {}", bind_addr);

      let app = waifu_calendar::http::router()?;
      let listener = tokio::net::TcpListener::bind(bind_addr).await.unwrap();
      axum::serve(listener, app).await.unwrap();
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

  let categories = characters.into_birthday_categories(now);

  if !categories.today.is_empty() {
    println!("Birthdays TODAY ({}):\n", now.date());

    categories.today.iter().for_each(|character| {
      println!("\t{}", character.name());
    });
  }

  if !categories.within_thirty_days.is_empty() {
    println!("\nUpcoming birthdays (next 30 days):\n");

    categories.within_thirty_days.iter().for_each(|character| {
        println!("{}", character_row(character, &now));
    });
  }

  if !categories.future.is_empty() {
    println!("\nFuture birthdays:\n");

    categories.future.iter().for_each(|character| {
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


