use crate::Torrent;
use url::Url;

/// Export crawl index to the RSS file
pub struct Feed {
    description: Option<String>,
    link: Option<String>,
    title: String,
    trackers: Option<Vec<Url>>,
}

impl Feed {
    pub fn init(
        title: String,
        description: Option<String>,
        link: Option<Url>,
        trackers: Option<Vec<Url>>,
    ) -> Self {
        Self {
            description: description.map(escape),
            link: link.map(|s| escape(s.to_string())),
            title: escape(title),
            trackers,
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
    pub fn push(&self, buffer: &mut String, torrent: Torrent) {
        buffer.push_str(&format!(
            "<item><guid>{}</guid><title>{}</title><link>{}</link>",
            torrent.info_hash,
            escape(
                torrent
                    .name
                    .as_ref()
                    .map(|b| b.to_string())
                    .unwrap_or("?".into()) // @TODO
            ),
            escape(torrent.magnet(self.trackers.as_ref()))
        ));

        buffer.push_str("<description>");
        buffer.push_str(&format!("{}\n{}", torrent.size(), torrent.files()));
        buffer.push_str("</description>");

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
}

fn escape(subject: String) -> String {
    subject
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace("'", "&apos;")
}
