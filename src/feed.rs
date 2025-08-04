use crate::format;
use std::collections::HashSet;
use url::Url;

/// Export crawl index to the RSS file
pub struct Feed {
    description: Option<String>,
    link: Option<String>,
    title: String,
    /// Valid, parsed from Url, ready-to-use address string donor
    trackers: Option<HashSet<String>>,
}

impl Feed {
    pub fn init(
        title: String,
        description: Option<String>,
        link: Option<Url>,
        trackers: Option<HashSet<Url>>,
    ) -> Self {
        Self {
            description: description.map(escape),
            link: link.map(|s| escape(s.to_string())),
            title: escape(title),
            trackers: trackers.map(|v| v.into_iter().map(|u| u.to_string()).collect()),
        }
    }

    pub fn transaction(&self, capacity: usize) -> String {
        let t = chrono::Utc::now().to_rfc2822();
        let mut b = String::with_capacity(capacity);

        b.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?><rss version=\"2.0\"><channel>");

        b.push_str("<pubDate>");
        b.push_str(&t);
        b.push_str("</pubDate>");

        b.push_str("<lastBuildDate>");
        b.push_str(&t);
        b.push_str("</lastBuildDate>");

        b.push_str("<title>");
        b.push_str(&self.title);
        b.push_str("</title>");

        if let Some(ref description) = self.description {
            b.push_str("<description>");
            b.push_str(description);
            b.push_str("</description>")
        }

        if let Some(ref link) = self.link {
            b.push_str("<link>");
            b.push_str(link);
            b.push_str("</link>")
        }
        b
    }

    /// Append `item` to the feed `channel`
    pub fn push(&self, buffer: &mut String, torrent: crate::storage::Torrent) {
        buffer.push_str(&format!(
            "<item><guid>{}</guid><title>{}</title><link>{}</link>",
            &torrent.info_hash,
            escape(
                torrent
                    .name
                    .as_ref()
                    .map(|b| b.to_string())
                    .unwrap_or("?".into()) // @TODO
            ),
            escape(self.magnet(&torrent.info_hash))
        ));

        if let Some(d) = item_description(torrent.length, torrent.files) {
            buffer.push_str("<description>");
            buffer.push_str(&escape(d));
            buffer.push_str("</description>")
        }

        buffer.push_str("<pubDate>");
        buffer.push_str(&torrent.time.to_rfc2822());
        buffer.push_str("</pubDate>");

        buffer.push_str("</item>")
    }

    /// Write final bytes
    pub fn commit(&self, mut buffer: String) -> String {
        buffer.push_str("</channel></rss>");
        buffer
    }

    // Tools

    fn magnet(&self, info_hash: &str) -> String {
        let mut b = if info_hash.len() == 40 {
            format!("magnet:?xt=urn:btih:{info_hash}")
        } else {
            todo!("info-hash v2 is not supported by librqbit")
        };
        if let Some(ref trackers) = self.trackers {
            for tracker in trackers {
                b.push_str("&tr=");
                b.push_str(&urlencoding::encode(tracker))
            }
        }
        b
    }
}

fn escape(subject: String) -> String {
    subject
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace("'", "&apos;")
}

fn item_description(size: Option<u64>, list: Option<Vec<crate::storage::File>>) -> Option<String> {
    if size.is_none() && list.is_none() {
        return None;
    }
    let mut b = Vec::with_capacity(list.as_ref().map(|l| l.len()).unwrap_or_default() + 1);
    if let Some(s) = size {
        b.push(format::bytes(s))
    }
    if let Some(files) = list {
        for file in files {
            b.push(format!(
                "{} ({})",
                file.name.as_deref().unwrap_or("?"), // @TODO invalid encoding
                format::bytes(file.length)
            ))
        }
    }
    Some(b.join("\n"))
}
