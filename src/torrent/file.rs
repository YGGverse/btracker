use rocket::serde::Serialize;

#[derive(Clone, Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct File {
    pub name: Option<String>,
    pub length: u64,
}

impl File {
    pub fn name(&self) -> String {
        self.name.as_deref().unwrap_or("?").into()
    }
    pub fn size(&self) -> String {
        super::size(self.length)
    }
}
