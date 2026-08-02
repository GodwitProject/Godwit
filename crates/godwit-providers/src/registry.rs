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
