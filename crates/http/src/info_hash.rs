use librqbit_core::Id20;
use std::str::FromStr;

pub struct Torrent(Id20);

impl Torrent {
    pub fn id20(&self) -> Id20 {
        self.0
    }
}

impl<'r> rocket::request::FromParam<'r> for Torrent {
    type Error = String;
    fn from_param(param: &'r str) -> Result<Self, Self::Error> {
        match param.strip_suffix(".torrent") {
            Some(id20_expected) => Ok(Self(
                <Id20 as FromStr>::from_str(id20_expected).map_err(|e| e.to_string())?,
            )),
            None => Err(format!("`{param}` is not valid torrent filename")),
        }
    }
}

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
            <Id20 as FromStr>::from_str(param).map_err(|e| e.to_string())?,
        ))
    }
}
