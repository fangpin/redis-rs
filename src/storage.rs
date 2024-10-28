use std::{
    collections::HashMap,
    time::{SystemTime, UNIX_EPOCH},
};

pub type ValueType = (String, Option<u128>);

pub struct Storage {
    // key -> (value, (insert/update time, expire milli seconds))
    set: HashMap<String, ValueType>,
}

#[inline]
pub fn now_in_millis() -> u128 {
    let start = SystemTime::now();
    let duration_since_epoch = start.duration_since(UNIX_EPOCH).unwrap();
    duration_since_epoch.as_millis()
}

impl Storage {
    pub fn new() -> Self {
        Storage {
            set: HashMap::new(),
        }
    }

    pub fn get(self: &mut Self, k: &str) -> Option<String> {
        match self.set.get(k) {
            Some((ss, expire_timestamp)) => match expire_timestamp {
                Some(expire_time_stamp) => {
                    if now_in_millis() > *expire_time_stamp {
                        self.set.remove(k);
                        None
                    } else {
                        Some(ss.clone())
                    }
                }
                _ => Some(ss.clone()),
            },
            _ => None,
        }
    }

    pub fn set(self: &mut Self, k: String, v: String) {
        self.set.insert(k, (v, None));
    }

    pub fn setx(self: &mut Self, k: String, v: String, expire_ms: u128) {
        self.set.insert(k, (v, Some(expire_ms + now_in_millis())));
    }

    pub fn del(self: &mut Self, k: String) {
        self.set.remove(&k);
    }

    pub fn keys(self: &Self) -> Vec<String> {
        self.set.keys().map(|x| x.clone()).collect()
    }
}
