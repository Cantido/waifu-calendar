use std::{collections::HashMap, sync::Arc, path::PathBuf};

use axum::{Router, extract::{Query, State}, response::{Response, IntoResponse, Html}, http::{StatusCode, header}, routing::get};
use handlebars::{Handlebars, DirectorySourceOptions, to_json};
use log::{info, error};
use moka::future::Cache;
use recloser::{AsyncRecloser, Recloser};
use serde::Serialize;
use time::{OffsetDateTime, Duration};
use tower_http::services::ServeFile;
use crate::{ics::BirthdayICalendar, Characters, Character, BirthdayCategories};

use anyhow::Result;

#[derive(Serialize)]
struct NoHandlebarsData;

struct AppState<'a> {
  handlebars: Handlebars<'a>,
  circuit_breaker: AsyncRecloser,
  cache: Cache<String, Vec<Character>>,
}

impl<'a> AppState<'a> {
  pub fn new(cache: Cache<String, Vec<Character>>, handlebars: Handlebars<'a>, circuit_breaker: AsyncRecloser) -> Self {
    Self {
      cache,
      handlebars,
      circuit_breaker,
    }
  }
}

pub fn router() -> Result<Router> {
  let mut assets_path = PathBuf::new();
  assets_path.push(std::env::var("WAIFU_ASSETS").unwrap_or(".".to_string()));

  info!("Loading assets from {:?}", assets_path);

  let mut handlebars = Handlebars::new();
  handlebars.set_strict_mode(true);
  handlebars.register_templates_directory(assets_path.join("templates"), DirectorySourceOptions::default())?;

  let circuit_breaker = AsyncRecloser::from(Recloser::default());

  let cache = Cache::builder()
    .weigher(|_key, value: &Vec<Character>| -> u32 {
      value.len().try_into().unwrap_or(u32::MAX)
    })
    .max_capacity(1024 * 1024)
    .time_to_live(std::time::Duration::from_secs(15 * 60))
    .build();

  let router =
    Router::new()
      .route("/", get(get_index))
      .route_service("/assets/pico.min.css", ServeFile::new(assets_path.join("assets/pico.min.css")))
      .route_service("/assets/frieren.jpg", ServeFile::new(assets_path.join("assets/frieren.jpg")))
      .route_service("/humans.txt", ServeFile::new(assets_path.join("assets/humans.txt")))
      .route("/ics", get(get_birthday_ics))
      .route("/cal", get(get_birthday_html))
      .with_state(Arc::new(AppState::new(cache, handlebars, circuit_breaker)));

  Ok(router)
}

async fn get_index(State(state): State<Arc<AppState<'_>>>) -> Result<Response, Response> {
  let data: HashMap<String, String> = HashMap::new();
  let body = state.handlebars.render("index", &data)
    .map_err(|_| render_internal_server_error(&state))?;

  Ok((
    Html::from(body)
  ).into_response())
}

#[derive(Debug, Serialize)]
struct CharacterHtml {
  name: String,
  til_next_iso: String,
  til_next_rounded: String,
  birthday: String,
  birthday_iso: String,
  next_occurrence: String,
}

impl CharacterHtml {
  pub fn new(character: &Character, now: &OffsetDateTime) -> Result<Self> {
    let next_occurrence = character.birthday().next_occurrence(&now.date())?;
    let til_next = character.birthday().til_next(&now);

    Ok(Self {
      next_occurrence: next_occurrence.to_string(),
      til_next_iso: duration_to_iso(&til_next),
      til_next_rounded: format!("{:.0}", til_next),
      name: character.name().to_string(),
      birthday: character.birthday().to_string(),
      birthday_iso: character.birthday().to_iso_string(),
    })
  }
}

fn duration_to_iso(dur: &Duration) -> String {
  let days = dur.whole_days();
  let hours = (*dur - Duration::days(dur.whole_days())).whole_hours();
  let minutes = (*dur - Duration::hours(dur.whole_hours())).whole_minutes();
  let seconds = (*dur - Duration::minutes(dur.whole_minutes())).whole_seconds();
  format!("P{}DT{}H{}M{}S", days, hours, minutes, seconds)
}

#[derive(Debug, Serialize)]
struct BirthdayHtml {
  username: String,
  today: Vec<CharacterHtml>,
  within_thirty_days: Vec<CharacterHtml>,
  future: Vec<CharacterHtml>,
}

impl BirthdayHtml {
  pub fn new(username: &str, categories: BirthdayCategories, now: &OffsetDateTime) -> Result<BirthdayHtml> {
    Ok(Self {
      username: username.to_string(),
      today: categories.today.iter().filter_map(|c| CharacterHtml::new(c, &now).ok()).collect(),
      within_thirty_days: categories.within_thirty_days.iter().filter_map(|c| CharacterHtml::new(c, &now).ok()).collect(),
      future: categories.future.iter().filter_map(|c| CharacterHtml::new(c, &now).ok()).collect(),
    })
  }
}

async fn get_birthday_html(State(state): State<Arc<AppState<'_>>>, Query(query): Query<HashMap<String, String>>) -> Result<Response, Response> {
  let cal: BirthdayHtml = {
    let username = query.get("username")
      .ok_or(StatusCode::UNPROCESSABLE_ENTITY.into_response())?;

    if username.is_empty() {
      return Err(StatusCode::UNPROCESSABLE_ENTITY.into_response());
    }

    let now = OffsetDateTime::now_utc();

    let cache_result = state.cache.get(username).await;
    let cache_hit = cache_result.is_some();

    let mut characters =
      if let Some(characters) = cache_result {
        Ok(characters)
      } else {
        state.circuit_breaker.call_with(should_melt, crate::get_waifu_birthdays(&username)).await
      }
      .map_err(|e| {
        match e {
          recloser::Error::Inner(err) => {
            match err.downcast::<crate::Error>() {
              Ok(crate::Error::UserNotFound(_)) => {
                let body = state.handlebars.render("user_not_found", &NoHandlebarsData {}).unwrap();
                (
                  StatusCode::NOT_FOUND,
                  Html::from(body),
                ).into_response()
              }
              Err(err) => {
                error!("Error contacting AniList: {:?}", err);
                let body = state.handlebars.render("internal_server_error", &NoHandlebarsData {}).unwrap();
                (
                  StatusCode::INTERNAL_SERVER_ERROR,
                  Html::from(body),
                ).into_response()
              }
              Ok(crate::Error::BadResponse) => {
                error!("Unknown error fetching from AniList");
                let body = state.handlebars.render("internal_server_error", &NoHandlebarsData {}).unwrap();
                (
                  StatusCode::INTERNAL_SERVER_ERROR,
                  Html::from(body),
                ).into_response()
              }
            }
          }
          recloser::Error::Rejected => {
            let body = state.handlebars.render("internal_server_error", &NoHandlebarsData {}).unwrap();
            (
              StatusCode::INTERNAL_SERVER_ERROR,
              Html::from(body),
            ).into_response()
          }
        }
      })?;

    characters.sort_by_upcoming(&now);

    if !cache_hit {
      state.cache.insert(username.to_string(), characters.clone()).await;
    }

    let categories = characters.into_birthday_categories(&now);

    BirthdayHtml::new(username, categories, &now)
      .map_err(|_| render_internal_server_error(&state))?
  };

  let body =
    state.handlebars.render("calendar", &to_json(cal))
      .map_err(|_| render_internal_server_error(&state))?;

  Ok((
    Html::from(body)
  ).into_response())
}



async fn get_birthday_ics(State(state): State<Arc<AppState<'_>>>, Query(query): Query<HashMap<String, String>>) -> Result<Response, Response> {
  let cal: String = {
    let username = query.get("username")
      .ok_or(StatusCode::UNPROCESSABLE_ENTITY.into_response())?;

    if username.is_empty() {
      return Err(StatusCode::UNPROCESSABLE_ENTITY.into_response());
    }

    let now = OffsetDateTime::now_utc();
    let cache_result = state.cache.get(username).await;
    let cache_hit = cache_result.is_some();

    let mut characters =
      if let Some(characters) = cache_result {
        Ok(characters)
      } else {
        state.circuit_breaker.call_with(should_melt, crate::get_waifu_birthdays(&username)).await
      }
      .map_err(|_| {
        let body = state.handlebars.render("user_not_found", &NoHandlebarsData {}).unwrap();
        (
          StatusCode::NOT_FOUND,
          Html::from(body),
        ).into_response()
      })?;

    if !cache_hit {
      state.cache.insert(username.to_string(), characters.clone()).await;
    }

    characters.sort_by_upcoming(&now);
    characters.to_ics(&now)
      .map_err(|_| render_internal_server_error(&state))?
  };

  Ok((
    [
      (header::CONTENT_DISPOSITION, "attachment; filename=\"birthdays.ics\""),
      (header::CONTENT_TYPE, "text/calendar"),
    ],
    cal,
  ).into_response())
}

fn render_internal_server_error(state: &Arc<AppState<'_>>) -> Response {
  let body = state.handlebars.render("internal_server_error", &NoHandlebarsData {}).unwrap();
  (
    StatusCode::INTERNAL_SERVER_ERROR,
    Html::from(body),
  ).into_response()
}

fn should_melt(err: &anyhow::Error) -> bool {
  let cast_err = err.downcast_ref::<crate::Error>();
  match cast_err {
    Some(crate::Error::UserNotFound(_)) => false,
    _ => true,
  }
}
