use helium_etl_lite::{
  settings::Settings,
  follower::Follower,
  migrate,
};
use slog::{self, o, Drain, Logger, info};
use std::{fs, fs::OpenOptions};
use structopt::StructOpt;
use tokio::time;
use tokio_postgres::{Client as PgClient, NoTls};

#[derive(Debug, StructOpt)]
#[structopt(name = "Helium Blockchain ETL Lite", about = "A Light ETL for the Helium Blockchain")]
pub struct Cli {
  #[structopt(subcommand)]
  cmd: Cmd,
}

#[derive(Debug, StructOpt)]
pub enum Cmd {
  Start,
  Migrate,
}

#[tokio::main]
async fn main() {
  let cli = Cli::from_args();
  let settings = Settings::new().unwrap();
  match cli.cmd {
    Cmd::Start => {
      let (client, connection) = tokio_postgres::connect(&settings.database_url, NoTls).await.unwrap();
      tokio::spawn(async move {
          if let Err(e) = connection.await {
              eprintln!("connection error: {}", e);
          }
      });     
      run(&settings, client).await;
    },
    Cmd::Migrate => {
      migrate::run(&settings).await;
      return
    },
  }  

  pub async fn run(settings: &Settings, client: PgClient) {
    let logger = start_logger(&settings);
    info!(logger, "hello!");
    let (shutdown_trigger, shutdown_listener) = triggered::trigger();
    tokio::spawn(async move {
        let _ = tokio::signal::ctrl_c().await;
        shutdown_trigger.trigger();
    }); 
    let mut follower = Follower::new(&settings, client, &logger, shutdown_listener.clone()).await.unwrap();
    info!(logger, "Starting blockchain follower at height: {}", follower.height);
    let mut interval = time::interval(time::Duration::from_secs(10));
    loop {
      tokio::select! {
        _ = shutdown_listener.clone() => {
          info!(logger, "Goodbye!");
          return
        },
        _ = interval.tick() => {
          follower.run().await;
        }
      }
    } 
  } 
}

fn start_logger(settings: &Settings) -> Logger {
  let log_dir = &settings.log.log_dir;
  let log_path = format!("{}/etl_lite.log", log_dir);
  fs::create_dir_all(log_dir).unwrap();
  let file = OpenOptions::new()
    .create(true)
    .write(true)
    .truncate(true)
    .open(log_path)
    .expect("opening file");
  let decorator = slog_term::PlainDecorator::new(file);
  let drain = slog_term::FullFormat::new(decorator)
    .use_custom_timestamp(slog_term::timestamp_local)
    .build()
    .fuse();
  let async_drain = slog_async::Async::new(drain)
    .build()
    .fuse();
  slog::Logger::root(async_drain, o!())
}
