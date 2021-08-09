use crate::*;
use config::{Config, File};
use serde::{de, Deserialize, Deserializer};
use std::path::PathBuf;
use http::uri::Uri;
use url::Url;

#[derive(Debug, Deserialize, Clone, Copy)]
pub enum EtlMode {
	Rewards,
	Challenges,
	Full,
	Filters,
}

#[derive(Debug, Deserialize)]
pub struct Log {
	pub log_dir: String,
}

#[derive(Debug, Deserialize)]
pub enum TxnTypes {
	RewardsV2,
	PocRequestV1,
	PocReceiptV1,
}

#[derive(Debug, Deserialize)]
pub struct Filter {
	// r#type: TxnTypes,
	accounts: Vec<String>,
	gateways: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct Database {
	pub user: String,
	pub password: String,
	pub host: String,
	pub db: String,
}

#[derive(Debug, Deserialize)]
pub struct Settings {
	pub log: Log,

  #[serde(deserialize_with = "deserialize_uri")]	
	pub node_addr: Uri,

	#[serde(deserialize_with = "deserialize_db_url")]
	pub database_url: Database,

	#[serde(deserialize_with = "deserialize_etl_mode")]
	pub mode: EtlMode,

	pub challenge_filters: Filter,

	pub rewards_filters: Filter,

}

impl Settings {
	pub fn new() -> Result<Self> {
		let mut con = Config::new();
		let path = PathBuf::from("config/settings.toml");
		con.merge(File::with_name(path.to_str().expect("file name")))?;
		con.try_into().map_err(|e| e.into())
	}
}

fn deserialize_uri<'de, D>(d: D) -> std::result::Result<Uri, D::Error>
where
    D: Deserializer<'de>,
{
    let uri_string = String::deserialize(d)?;
    match uri_string.parse() {
        Ok(uri) => Ok(uri),
        Err(err) => Err(de::Error::custom(format!("invalid uri: \"{}\"", err))),
    }
}

fn deserialize_etl_mode<'de, D>(d: D) -> std::result::Result<EtlMode, D::Error>
where
    D: Deserializer<'de>,
{
    let mode = match String::deserialize(d)?.to_lowercase().as_str() {
        "rewards" => EtlMode::Rewards,
        "challenges" => EtlMode::Challenges,
        unsupported => {
            return Err(de::Error::custom(format!(
                "unsupported etl mode: \"{}\"",
                unsupported
            )))
        }
    };
    Ok(mode)
}

fn deserialize_db_url<'de, D>(d: D) -> std::result::Result<Database, D::Error>
where
	D: Deserializer<'de>,
{
	let db_url_str = String::deserialize(d)?;
	let db_url = match Url::parse(&db_url_str) {
		Ok(url) => url,
		Err(_) => return Err(de::Error::custom(format!("invalid database url: {}", db_url_str))),
	};

	Ok(Database {
		user: db_url.username().to_string(),
		password: db_url.password().unwrap().to_string(),
		host: db_url.host().unwrap().to_string(),
		db: db_url.path_segments().map(|c| c.collect::<Vec<_>>()).unwrap()[0].to_string(),
	})
}