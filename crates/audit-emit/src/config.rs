#[derive(Debug, Clone)]
pub struct EmitterConfig {
    pub detail_max_bytes: usize,
    pub tags_max_pairs: usize,
    pub tag_value_max_bytes: usize,
}

impl Default for EmitterConfig {
    fn default() -> Self {
        Self {
            detail_max_bytes: 4096,
            tags_max_pairs: 32,
            tag_value_max_bytes: 256,
        }
    }
}
