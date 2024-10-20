use std::{collections::HashMap, time::Instant};

pub type ValueType = (String, Option<(Instant, u128)>);

pub struct Storage {
    // key -> (value, (insert/update time, expire milli seconds))
    set: HashMap<String, ValueType>,
}

impl Storage {
    pub fn new() -> Self {
        Storage {
            set: HashMap::new(),
        }
    }

    pub fn get(self: &Self, k: &str) -> Option<String> {
        match self.set.get(k) {
            Some((ss, time_info)) => match time_info {
                Some((instant, ms)) => {
                    if instant.elapsed().as_millis() > *ms {
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
        self.set.insert(k, (v, Some((Instant::now(), expire_ms))));
    }

    pub fn keys(self: &Self) -> Vec<String> {
        self.set.keys().map(|x| x.clone()).collect()
    }
}
