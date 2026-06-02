use librqbit_core::torrent_metainfo::TorrentMetaV1Owned;
use plurify::Plurify;
use url::Url;

pub fn files(meta: &TorrentMetaV1Owned) -> String {
    let total = meta.info.files.as_ref().map(|f| f.len()).unwrap_or(1);
    format!("{total} {}", total.plurify(&["file", "files", "files"]))
}

pub fn size(value: u64) -> String {
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

pub fn total(meta: &TorrentMetaV1Owned) -> String {
    size(
        meta.info
            .files
            .as_ref()
            .map(|files| files.iter().map(|file| file.length).sum::<u64>())
            .unwrap_or_default()
            + meta.info.length.unwrap_or_default(),
    )
}

pub fn magnet(meta: &TorrentMetaV1Owned, trackers: Option<&Vec<Url>>) -> String {
    let mut b = format!("magnet:?xt=urn:btih:{}", meta.info_hash.as_string());
    if let Some(ref n) = meta.info.name {
        b.push_str("&dn=");
        b.push_str(&urlencoding::encode(&n.to_string()))
    }
    if let Some(t) = trackers {
        for tracker in t {
            b.push_str("&tr=");
            b.push_str(&urlencoding::encode(tracker.as_str()))
        }
    }
    b
}
