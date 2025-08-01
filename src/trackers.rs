use std::{collections::HashSet, str::FromStr};
use url::Url;

pub struct Trackers(HashSet<Url>);

impl Trackers {
    pub fn init(trackers: &Vec<String>) -> anyhow::Result<Self> {
        let mut t = HashSet::with_capacity(trackers.len());
        for tracker in trackers {
            t.insert(Url::from_str(tracker)?);
        }
        Ok(Self(t))
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    pub fn list(&self) -> &HashSet<Url> {
        &self.0
    }
}
