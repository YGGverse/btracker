use anyhow::Result;
use bendy::decoding::Decoder;
use librqbit::dht::Id20;
use std::io;

/// Try parse info-hash from the given source,
/// convert bytes to valid `InfoHash` v1 array on success.
pub async fn get(source: &str, capacity: usize) -> Result<Vec<Id20>> {
    let mut i = Vec::with_capacity(capacity);

    let bytes = if source.starts_with("http://") {
        reqwest::get(source).await?.bytes().await?.into()
    } else {
        tokio::fs::read(source.trim_start_matches("file://")).await?
    };

    let mut decoder = Decoder::new(&bytes);

    let root_object = decoder
        .next_object()?
        .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "empty"))?;

    let mut outer_dict = root_object.try_into_dictionary()?;

    while let Some(pair) = outer_dict.next_pair().unwrap_or(None) {
        if pair.0 == b"files" {
            let mut files_dict = pair
                .1
                .try_into_dictionary()
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
            while let Some(file_pair) = files_dict.next_pair().unwrap_or(None) {
                i.push(Id20::from_bytes(file_pair.0)?)
            }
        }
    }

    Ok(i)
}
