use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ItemId(pub String);

impl ItemId {
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ItemMetadata {
    pub id: ItemId,
    pub difficulty: f64,
    pub discrimination: f64,
    pub content_domain: String,
    pub exposure_count: u64,
}

impl ItemMetadata {
    #[must_use]
    pub fn new(id: ItemId, difficulty: f64, discrimination: f64, content_domain: String) -> Self {
        Self {
            id,
            difficulty,
            discrimination,
            content_domain,
            exposure_count: 0,
        }
    }
}

#[derive(Debug, Clone, thiserror::Error)]
#[non_exhaustive]
pub enum PoolError {
    #[error("item pool is empty")]
    Empty,
    #[error("requested {requested} items but pool contains only {available}")]
    InsufficientItems { requested: usize, available: usize },
    #[error("duplicate item ID: {0:?}")]
    DuplicateId(ItemId),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemPool {
    items: Vec<ItemMetadata>,
}

impl ItemPool {
    pub fn new(items: Vec<ItemMetadata>) -> Result<Self, PoolError> {
        if items.is_empty() {
            return Err(PoolError::Empty);
        }
        for (i, item) in items.iter().enumerate() {
            for other in &items[i + 1..] {
                if item.id == other.id {
                    return Err(PoolError::DuplicateId(item.id.clone()));
                }
            }
        }
        Ok(Self { items })
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    #[must_use]
    pub fn items(&self) -> &[ItemMetadata] {
        &self.items
    }

    #[must_use]
    pub fn get(&self, id: &ItemId) -> Option<&ItemMetadata> {
        self.items.iter().find(|i| &i.id == id)
    }

    pub fn record_exposure(&mut self, id: &ItemId) {
        if let Some(item) = self.items.iter_mut().find(|i| &i.id == id) {
            item.exposure_count += 1;
        }
    }

    #[must_use]
    pub fn domains(&self) -> Vec<&str> {
        let mut ds: Vec<&str> = self
            .items
            .iter()
            .map(|i| i.content_domain.as_str())
            .collect();
        ds.sort_unstable();
        ds.dedup();
        ds
    }

    #[must_use]
    pub fn items_in_domain(&self, domain: &str) -> Vec<&ItemMetadata> {
        self.items
            .iter()
            .filter(|i| i.content_domain == domain)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_items() -> Vec<ItemMetadata> {
        vec![
            ItemMetadata::new(ItemId::new("t1"), 0.5, 1.0, "math".into()),
            ItemMetadata::new(ItemId::new("t2"), 0.7, 1.2, "math".into()),
            ItemMetadata::new(ItemId::new("t3"), 0.3, 0.8, "code".into()),
            ItemMetadata::new(ItemId::new("t4"), 0.9, 1.5, "code".into()),
            ItemMetadata::new(ItemId::new("t5"), 0.6, 1.1, "reasoning".into()),
        ]
    }

    #[test]
    fn pool_creation_succeeds() {
        let pool = ItemPool::new(sample_items()).unwrap();
        assert_eq!(pool.len(), 5);
    }

    #[test]
    fn pool_rejects_empty() {
        let err = ItemPool::new(vec![]).unwrap_err();
        assert!(matches!(err, PoolError::Empty));
    }

    #[test]
    fn pool_rejects_duplicates() {
        let items = vec![
            ItemMetadata::new(ItemId::new("t1"), 0.5, 1.0, "math".into()),
            ItemMetadata::new(ItemId::new("t1"), 0.7, 1.2, "math".into()),
        ];
        let err = ItemPool::new(items).unwrap_err();
        assert!(matches!(err, PoolError::DuplicateId(_)));
    }

    #[test]
    fn domains_are_deduplicated() {
        let pool = ItemPool::new(sample_items()).unwrap();
        let domains = pool.domains();
        assert_eq!(domains, vec!["code", "math", "reasoning"]);
    }

    #[test]
    fn items_in_domain_filters() {
        let pool = ItemPool::new(sample_items()).unwrap();
        let math = pool.items_in_domain("math");
        assert_eq!(math.len(), 2);
    }

    #[test]
    fn record_exposure_increments() {
        let mut pool = ItemPool::new(sample_items()).unwrap();
        let id = ItemId::new("t1");
        pool.record_exposure(&id);
        pool.record_exposure(&id);
        assert_eq!(pool.get(&id).unwrap().exposure_count, 2);
    }
}
