use crate::core::buffer::Buffer;
use std::collections::HashMap;

#[derive(Clone, Eq, Hash, PartialEq)]
pub struct CacheKey {
    path: String,
    width: u32,
    height: u32,
}

impl CacheKey {
    pub fn new(path: &str, width: u32, height: u32) -> Self {
        Self {
            path: path.to_string(),
            width,
            height,
        }
    }
}

pub struct Cache {
    buffers: HashMap<CacheKey, Buffer>,
}

impl Default for Cache {
    fn default() -> Self {
        Self::new()
    }
}

impl Cache {
    pub fn new() -> Self {
        Self {
            buffers: HashMap::new(),
        }
    }

    pub fn get(&self, key: &CacheKey) -> Option<&Buffer> {
        self.buffers.get(key)
    }

    pub fn insert(&mut self, key: CacheKey, buffer: Buffer) {
        self.buffers.insert(key, buffer);
    }
}
