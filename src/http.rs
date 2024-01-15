use std::{collections::HashMap, sync::Arc};

use axum::{Router, extract::{Query, State}, response::{Response, IntoResponse, Html}, http::{StatusCode, header}, routing::get};
use handlebars::{Handlebars, DirectorySourceOptions, to_json};
use serde::Serialize;
use time::OffsetDateTime;
use tower_http::services::ServeFile;
use crate::{ics::BirthdayICalendar, Characters, Character, BirthdayCategories};

use anyhow::Result;

#[derive(Serialize)]
struct NoHandlebarsData;

struct AppState<'a> {
  handlebars: Handlebars<'a>,
}

impl<'a> AppState<'a> {
  pub fn new(handlebars: Handlebars<'a>) -> Self {
    Self {
      handlebars
    }
  }
}

pub fn router() -> Result<Router> {
  let mut handlebars = Handlebars::new();
  handlebars.set_strict_mode(true);
  handlebars.register_templates_directory("templates", DirectorySourceOptions::default())?;

  let router =
    Router::new()
      .route("/", get(get_index))
      .route_service("/assets/pico.min.css", ServeFile::new("assets/pico.min.css"))
      .route("/ics", get(get_birthday_ics))
      .route("/cal", get(get_birthday_html))
      .with_state(Arc::new(AppState::new(handlebars)));

  Ok(router)
}

async fn get_index(State(state): State<Arc<AppState<'_>>>) -> Result<Response, Response> {
  let data: HashMap<String, String> = HashMap::new();
  let body = state.handlebars.render("index", &data)
    .map_err(|e| {
      println!("{:?}", e);
      StatusCode::INTERNAL_SERVER_ERROR.into_response()
    })?;

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
  next_occurrence: String,
}

impl CharacterHtml {
  pub fn new(character: &Character, now: &OffsetDateTime) -> Result<Self> {
    let next_occurrence = character.birthday().next_occurrence(&now.date())?;
    let til_next = character.birthday().til_next(&now);

    Ok(Self {
      next_occurrence: next_occurrence.to_string(),
      til_next_iso: til_next.to_string(),
      til_next_rounded: format!("{:.0}", til_next),
      name: character.name().to_string(),
      birthday: character.birthday().to_string(),
    })
  }
}

#[derive(Debug, Serialize)]
struct BirthdayHtml {
  today: Vec<CharacterHtml>,
  within_thirty_days: Vec<CharacterHtml>,
  future: Vec<CharacterHtml>,
}

impl BirthdayHtml {
  pub fn new(categories: BirthdayCategories, now: &OffsetDateTime) -> Result<BirthdayHtml> {
    Ok(Self {
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
    let mut characters = crate::get_waifu_birthdays(&username).await
      .map_err(|_| {
        let body = state.handlebars.render("user_not_found", &NoHandlebarsData {}).unwrap();
        (
          StatusCode::NOT_FOUND,
          Html::from(body),
        ).into_response()
      })?;

    characters.sort_by_upcoming(&now);

    let categories = characters.into_birthday_categories(&now);

    BirthdayHtml::new(categories, &now)
      .map_err(|e| {
        println!("{:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR.into_response()
      })?
  };

  let body =
    state.handlebars.render("calendar", &to_json(cal))
      .map_err(|e| {
        println!("{:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR.into_response()
      })?;

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
    let mut characters = crate::get_waifu_birthdays(&username).await
      .map_err(|_| {
        let body = state.handlebars.render("user_not_found", &NoHandlebarsData {}).unwrap();
        (
          StatusCode::NOT_FOUND,
          Html::from(body),
        ).into_response()
      })?;

    characters.sort_by_upcoming(&now);
    characters.to_ics(&now)
      .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?
  };

  Ok((
    [
      (header::CONTENT_DISPOSITION, "attachment; filename=\"birthdays.ics\""),
      (header::CONTENT_TYPE, "text/calendar"),
    ],
    cal,
  ).into_response())
}
