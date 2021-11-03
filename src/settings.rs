use crate::*;
use config::{Config, File};
use serde::{de, Deserialize, Deserializer};
use std::path::PathBuf;
use http::uri::Uri;

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
pub struct Settings {
	pub log: Log,

  #[serde(deserialize_with = "deserialize_uri")]	
	pub node_addr: Uri,

	pub database_url: String,

	#[serde(deserialize_with = "deserialize_etl_mode")]
	pub mode: EtlMode,

	#[serde(deserialize_with = "deserialize_backfill")]	
	pub backfill: bool,

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

fn deserialize_backfill<'de, D>(d: D) -> std::result::Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    let backfill = match String::deserialize(d)?.to_lowercase().as_str() {
        "true" => true,
        "false" => false,
        unsupported => {
            return Err(de::Error::custom(format!(
                "unsupported etl mode: \"{}\"",
                unsupported
            )))
        }
    };
    Ok(backfill)
}