use rocket::serde::Serialize;
use url::Url;

#[derive(Clone, Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct Meta {
    pub canonical: Option<Url>,
    pub description: Option<String>,
    pub format_time: String,
    pub title: String,
    /// * use vector to keep the order from the arguments list
    pub trackers: Option<Vec<Url>>,
    pub version: String,
}
