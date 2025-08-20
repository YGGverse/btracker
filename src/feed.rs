mod link;

use crate::Torrent;
use link::Link;
use url::Url;

/// Export crawl index to the RSS file
pub struct Feed {
    buffer: String,
    canonical: Link,
}

impl Feed {
    pub fn new(
        title: &str,
        description: Option<&str>,
        canonical: Option<Url>,
        capacity: usize,
    ) -> Self {
        let t = chrono::Utc::now().to_rfc2822();
        let mut buffer = String::with_capacity(capacity);

        buffer.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?><rss version=\"2.0\"><channel>");

        buffer.push_str("<pubDate>");
        buffer.push_str(&t);
        buffer.push_str("</pubDate>");

        buffer.push_str("<lastBuildDate>");
        buffer.push_str(&t);
        buffer.push_str("</lastBuildDate>");

        buffer.push_str("<title>");
        buffer.push_str(title);
        buffer.push_str("</title>");

        if let Some(d) = description {
            buffer.push_str("<description>");
            buffer.push_str(d);
            buffer.push_str("</description>")
        }

        if let Some(ref c) = canonical {
            // @TODO required the RSS specification!
            buffer.push_str("<link>");
            buffer.push_str(c.as_str());
            buffer.push_str("</link>")
        }

        Self {
            buffer,
            canonical: Link::from_url(canonical),
        }
    }

    /// Append `item` to the feed `channel`
    pub fn push(&mut self, torrent: Torrent) {
        self.buffer.push_str(&format!(
            "<item><guid>{}</guid><title>{}</title><link>{}</link>",
            torrent.info_hash,
            escape(
                &torrent
                    .name
                    .as_ref()
                    .map(|b| b.to_string())
                    .unwrap_or("?".into()) // @TODO
            ),
            self.canonical.link(&torrent.info_hash)
        ));

        self.buffer.push_str("<description>");
        self.buffer
            .push_str(&format!("size: {}", size(torrent.size)));
        if let Some(f) = torrent.files() {
            self.buffer.push_str(&format!(" / files: {f}"));
        }
        self.buffer.push_str("</description>");

        self.buffer.push_str("<pubDate>");
        self.buffer.push_str(&torrent.time.to_rfc2822());
        self.buffer.push_str("</pubDate>");

        self.buffer.push_str("</item>")
    }

    /// Write final bytes
    pub fn commit(mut self) -> String {
        self.buffer.push_str("</channel></rss>");
        self.buffer
    }
}

// @TODO use tera filters?
// https://keats.github.io/tera/docs/#built-in-filters

fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace("'", "&apos;")
}

fn size(value: u64) -> String {
    const KB: f32 = 1024.0;
    const MB: f32 = KB * KB;
    const GB: f32 = MB * KB;

    let f = value as f32;

    if f < KB {
        format!("{value} B")
    } else if f < MB {
        format!("{:.2} KB", f / KB)
    } else if f < GB {
        format!("{:.2} MB", f / MB)
    } else {
        format!("{:.2} GB", f / GB)
    }
}
