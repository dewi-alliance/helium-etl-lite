use crate::*;
use tokio_postgres::NoTls;

mod embedded {
    use refinery::embed_migrations;
    embed_migrations!("./migrations");
}

pub async fn run(settings: &Settings) {
  let (mut client, connection) = tokio_postgres::connect(&settings.database_url, NoTls).await.unwrap();
  tokio::spawn(async move {
      if let Err(e) = connection.await {
          eprintln!("connection error: {}", e);
      }
  });
  let res = embedded::migrations::runner().run_async(&mut client).await.unwrap();
  println!("{:?}", res);
}