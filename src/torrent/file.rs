#[derive(Clone, Debug, rocket::serde::Serialize)]
#[serde(crate = "rocket::serde")]
pub struct File {
    pub path: Option<std::path::PathBuf>,
    pub length: u64,
}

impl File {
    pub fn path(&self) -> String {
        self.path
            .as_ref()
            .map(|p| p.to_string_lossy().into())
            .unwrap_or("?".into())
    }
}
