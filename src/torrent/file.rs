use rocket::serde::Serialize;

#[derive(Clone, Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct File {
    pub name: Option<String>,
    pub length: u64,
}
