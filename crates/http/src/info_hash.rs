use librqbit_core::Id20;

pub struct InfoHash(Id20);

impl InfoHash {
    pub fn id20(&self) -> Id20 {
        self.0
    }
    pub fn bytes20(&self) -> [u8; 20] {
        self.0.0
    }
}

impl<'r> rocket::request::FromParam<'r> for InfoHash {
    type Error = String;
    fn from_param(param: &'r str) -> Result<Self, Self::Error> {
        Ok(Self(
            <Id20 as std::str::FromStr>::from_str(param).map_err(|e| e.to_string())?,
        ))
    }
}
