use std::sync::Mutex;

use lru::LruCache;
use once_cell::sync::Lazy;
use sled::{IVec, Tree};

use crate::utils::AnyResult;

/// URL -> ETAG
static URL_TO_ETAG: Lazy<Mutex<LruCache<String, String>>> =
    Lazy::new(|| Mutex::new(LruCache::unbounded()));

/// ETAG -> HTTP Response Body
static CACHE_DATA_DB: Lazy<Tree> = Lazy::new(|| {
    let result = crate::command::database()
        .open_tree("http-cache-data")
        .unwrap();
    result.clear().expect("failed to clean http cache on disk"); // clean previous cache
    result
});

const CACHE_LIMIT: usize = 10240;

pub fn init() -> AnyResult {
    Lazy::force(&CACHE_DATA_DB);
    Lazy::force(&URL_TO_ETAG);
    Ok(())
}

pub fn get(url: &String) -> AnyResult<Option<(String, IVec)>> {
    let mut cache = URL_TO_ETAG.lock().unwrap();

    let etag: &String = match cache.get(url) {
        Some(x) => x,
        None => return Ok(None),
    };

    let data: IVec = match CACHE_DATA_DB.get(etag.as_bytes())? {
        Some(x) => x,
        None => return Ok(None),
    };

    Ok(Some((etag.clone(), data)))
}

pub fn set(url: String, data: IVec, etag: String) -> AnyResult {
    CACHE_DATA_DB.insert(etag.as_bytes(), data)?;

    let mut cache = URL_TO_ETAG.lock().unwrap();

    cache.put(url, etag);

    if cache.len() > CACHE_LIMIT {
        let (_, etag) = cache.pop_lru().unwrap();
        if let Err(e) = CACHE_DATA_DB.remove(etag.as_bytes()) {
            error!("failed to remove cache item from disk: {:?}", e);
        }
    }

    drop(cache);

    Ok(())
}
