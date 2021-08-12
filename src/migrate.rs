use crate::*;
use tokio_postgres::NoTls;

mod embedded {
    use refinery::embed_migrations;
    embed_migrations!("./migrations");
}

pub async fn run(settings: &Settings) {
	let s = format!("host={} user={} password={} dbname={} ", 
		settings.database_url.host, settings.database_url.user, settings.database_url.password, settings.database_url.db);

	let (mut client, connection) = tokio_postgres::connect(&s, NoTls).await.unwrap();

  tokio::spawn(async move {
      if let Err(e) = connection.await {
          eprintln!("connection error: {}", e);
      }
  });
  let res = embedded::migrations::runner().run_async(&mut client).await.unwrap();
  println!("{:?}", res);


}