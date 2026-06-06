// Central error type for the backend.
//
// Every Tauri command returns `Result<T, AppError>`. Tauri requires the error
// type to be `Serialize`, so this enum serialises to a plain human-readable
// string that the frontend can surface directly to the user. `From` conversions
// for the common library errors let command bodies use the `?` operator freely.

use serde::Serialize;
use serde::Serializer;

#[derive(Debug, thiserror::Error)]
pub enum AppError
{
	#[error("database error: {0}")]
	Database(String),

	#[error("filesystem error: {0}")]
	Io(String),

	#[error("serialization error: {0}")]
	Serialization(String),

	#[error("network error: {0}")]
	Network(String),

	#[error("invalid input: {0}")]
	Validation(String),

	#[error("HMRC is not configured: {0}")]
	HmrcNotConfigured(String),

	#[error("not found: {0}")]
	NotFound(String),

	#[error("internal error: {0}")]
	Internal(String),
}

// Serialise as a flat string so the React layer receives a readable message
// instead of a tagged enum object it would have to decode.
impl Serialize for AppError
{
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		serializer.serialize_str(&self.to_string())
	}
}

// Convert a rusqlite error so database calls can use `?`.
impl From<rusqlite::Error> for AppError
{
	fn from(error: rusqlite::Error) -> Self
	{
		AppError::Database(error.to_string())
	}
}

// Convert a std::io error so filesystem calls can use `?`.
impl From<std::io::Error> for AppError
{
	fn from(error: std::io::Error) -> Self
	{
		AppError::Io(error.to_string())
	}
}

// Convert a serde_json error so settings/JSON calls can use `?`.
impl From<serde_json::Error> for AppError
{
	fn from(error: serde_json::Error) -> Self
	{
		AppError::Serialization(error.to_string())
	}
}

// Convert a reqwest error so HMRC HTTP calls can use `?`.
impl From<reqwest::Error> for AppError
{
	fn from(error: reqwest::Error) -> Self
	{
		AppError::Network(error.to_string())
	}
}

// A convenient project-wide Result alias.
pub type AppResult<T> = std::result::Result<T, AppError>;
