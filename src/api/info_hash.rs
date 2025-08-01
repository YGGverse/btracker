pub enum InfoHash {
    V1([u8; 20]),
}

impl std::fmt::Display for InfoHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::V1(i) => write!(
                f,
                "{}",
                i.iter().map(|b| format!("{b:02x}")).collect::<String>()
            ),
        }
    }
}
