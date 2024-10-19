use std::collections::HashMap;

pub struct Storage {
    set: HashMap<String, String>,
}

impl Storage {
    pub fn new() -> Self {
        Storage {
            set: HashMap::new(),
        }
    }

    pub fn get(self: &Self, k: &str) -> Option<&String> {
        self.set.get(k)
    }

    pub fn set(self: &mut Self, k: String, v: String) {
        self.set.insert(k, v);
    }
}
