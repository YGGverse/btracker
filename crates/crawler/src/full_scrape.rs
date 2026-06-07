use anyhow::{Result, bail};
use bendy::decoding::Decoder;
use librqbit::dht::Id20;
use log::*;
use reqwest::{Client, Proxy, header::*};
use std::{io, time::Duration};

/// Try parse info-hash from the given source,
/// convert bytes to valid `InfoHash` v1 array on success.
pub async fn get(
    source_url: &str,
    capacity: usize,
    timeout: Duration,
    compression_method: &str,
    http_proxy_url: Option<&str>,
) -> Result<Vec<Id20>> {
    let mut i = Vec::with_capacity(capacity);

    let bytes = if source_url.starts_with("http") {
        trace!("build full scrape request to `{source_url}`...");
        let client = Client::builder().timeout(timeout);
        let request = match http_proxy_url {
            Some(p) => client.proxy(if p.starts_with("https") {
                trace!("applying HTTPs proxy `{p}` to `{source_url}`...");
                Proxy::https(p)?
            } else {
                trace!("applying HTTP proxy `{p}` to `{source_url}`...");
                Proxy::http(p)?
            }),
            None => {
                trace!("sending direct request to `{source_url}`...");
                client
            }
        }
        .build()?
        .get(source_url);

        trace!("sending full scrape request to `{source_url}`...");
        let response = if compression_method.is_empty() {
            trace!("disable compression for `{source_url}`...");
            request
        } else {
            trace!("set `{compression_method}` compression method for `{source_url}`...");
            request.header(ACCEPT_ENCODING, compression_method)
        }
        .send()
        .await?;

        if !response.status().is_success() {
            bail!(
                "HTTP request to `{source_url}` failed: {}",
                response.status()
            )
        }

        trace!("HTTP response from `{source_url}` received successfully, reading the bytes...");
        let res_bytes = response.bytes().await?;
        res_bytes.to_vec()
    } else {
        trace!("begin full scrape request from `{source_url}`...");
        tokio::fs::read(source_url.trim_start_matches("file://")).await?
    };

    let mut decoder = Decoder::new(&bytes);

    let root_object = decoder
        .next_object()?
        .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "empty"))?;

    trace!("dictionary parse begin...");
    let mut outer_dict = root_object.try_into_dictionary()?;

    while let Some(pair) = outer_dict.next_pair().unwrap_or(None) {
        if pair.0 == b"files" {
            trace!("the `files` index found...");
            let mut files_dict = pair
                .1
                .try_into_dictionary()
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
            while let Some(file_pair) = files_dict.next_pair().unwrap_or(None) {
                let id20 = Id20::from_bytes(file_pair.0)?;
                trace!("push `{}` to queue...", id20.as_string());
                i.push(id20)
            }
            break;
        }
    }

    Ok(i)
}
