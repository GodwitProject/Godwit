use crate::adapter::Adapter;
use godwit_core::Protocol;
use std::{collections::HashMap, sync::Arc};

pub struct AdapterRegistry {
    adapters: HashMap<Protocol, Arc<dyn Adapter>>,
}

impl AdapterRegistry {
    pub fn new() -> Self {
        Self {
            adapters: HashMap::new(),
        }
    }

    pub fn register(&mut self, protocol: Protocol, adapter: Arc<dyn Adapter>) {
        self.adapters.insert(protocol, adapter);
    }

    pub fn get(&self, protocol: &Protocol) -> Option<Arc<dyn Adapter>> {
        self.adapters.get(protocol).cloned()
    }
}

impl Default for AdapterRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::openai::OpenAiAdapter;
    use std::sync::Arc;

    #[test]
    fn registry_stores_adapter() {
        let mut registry = AdapterRegistry::new();
        registry.register(Protocol::openai(), Arc::new(OpenAiAdapter::new("", "")));
        assert!(registry.get(&Protocol::openai()).is_some());
    }

    #[test]
    fn get_returns_none_for_unknown_protocol() {
        let registry = AdapterRegistry::new();
        assert!(registry.get(&Protocol::anthropic()).is_none());
    }

    #[test]
    fn register_overwrites_existing_adapter() {
        let mut registry = AdapterRegistry::new();
        let first: Arc<dyn Adapter> = Arc::new(OpenAiAdapter::new("", ""));
        let second: Arc<dyn Adapter> = Arc::new(OpenAiAdapter::new("", ""));
        registry.register(Protocol::openai(), first.clone());
        let before = registry.get(&Protocol::openai()).expect("adapter stored");
        assert!(Arc::ptr_eq(&before, &first));
        registry.register(Protocol::openai(), second.clone());
        let after = registry.get(&Protocol::openai()).expect("adapter stored");
        assert!(Arc::ptr_eq(&after, &second));
        assert!(!Arc::ptr_eq(&after, &first));
    }

    #[test]
    fn default_creates_empty_registry() {
        let registry: AdapterRegistry = Default::default();
        assert!(registry.get(&Protocol::openai()).is_none());
    }
}
