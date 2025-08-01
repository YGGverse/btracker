mod info_hash;
use info_hash::InfoHash;

/// Parse infohash from the source filepath,
/// decode hash bytes to `InfoHash` array on success.
///
/// * return `None` if the `path` is not reachable
pub fn get(path: &str, capacity: usize) -> Option<Vec<InfoHash>> {
    use std::io::Read;
    if !path.ends_with(".bin") {
        todo!("Only sources in the `.bin` format are supported!")
    }
    if path.contains("://") {
        todo!("URL source format is not supported!")
    }
    const L: usize = 20; // v1 only
    let mut r = Vec::with_capacity(capacity);
    let mut f = std::fs::File::open(path).ok()?;
    loop {
        let mut b = [0; L];
        if f.read(&mut b).ok()? != L {
            break;
        }
        r.push(InfoHash::V1(b))
    }
    Some(r)
}

#[test]
fn test() {
    use std::fs;

    #[cfg(not(any(target_os = "linux", target_os = "macos",)))]
    {
        todo!()
    }

    const C: usize = 2;

    const P0: &str = "/tmp/yggtrackerd-api-test-0.bin";
    const P1: &str = "/tmp/yggtrackerd-api-test-1.bin";
    const P2: &str = "/tmp/yggtrackerd-api-test-2.bin";

    fs::write(P0, vec![]).unwrap();
    fs::write(P1, vec![1; 40]).unwrap(); // 20 + 20 bytes

    assert!(get(P0, C).is_some_and(|b| b.is_empty()));
    assert!(get(P1, C).is_some_and(|b| b.len() == 2));
    assert!(get(P2, C).is_none());

    fs::remove_file(P0).unwrap();
    fs::remove_file(P1).unwrap();
}
