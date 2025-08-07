pub fn bytes(value: u64) -> String {
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

pub fn magnet(info_hash: &str, trackers: Option<&Vec<url::Url>>) -> String {
    let mut b = if info_hash.len() == 40 {
        format!("magnet:?xt=urn:btih:{info_hash}")
    } else {
        todo!("info-hash v2 yet not supported") // librqbit_core::hash_id::Id
    };
    if let Some(t) = trackers {
        for tracker in t {
            b.push_str("&tr=");
            b.push_str(&urlencoding::encode(tracker.as_str()))
        }
    }
    b
}
