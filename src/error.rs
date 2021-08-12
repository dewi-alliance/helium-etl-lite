use thiserror::Error;

pub type Result<T = ()> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
  #[error("tokio-postgres error: {0}")]
  PgError(#[from] tokio_postgres::Error),	
	#[error("config error")]
	Config(#[from] config::ConfigError),
	#[error("custom error")]
	Custom(String),
	#[error("helium_jsonrpc error")]
	JrpcError(#[from] helium_jsonrpc::Error),
}

impl Error {

    pub fn custom<T: ToString>(msg: T) -> Error {
        Error::Custom(msg.to_string())
    }
}