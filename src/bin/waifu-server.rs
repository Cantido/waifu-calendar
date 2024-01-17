use log::info;

use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
  env_logger::init();

  let bind_addr = "0.0.0.0:8080";

  info!("starting Waifu Calendar on {}", bind_addr);

  let app = waifu_calendar::http::router()?;
  let listener = tokio::net::TcpListener::bind(bind_addr).await.unwrap();
  axum::serve(listener, app).await?;

  Ok(())
}
