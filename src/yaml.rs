use serde::{Deserialize, Serialize};

pub use serde_norway::Value;

pub type Error = serde_norway::Error;

pub fn from_str<'de, T>(content: &'de str) -> Result<T, Error>
where
    T: Deserialize<'de>,
{
    serde_norway::from_str(content)
}

pub fn to_string<T>(value: &T) -> Result<String, Error>
where
    T: Serialize + ?Sized,
{
    serde_norway::to_string(value)
}
