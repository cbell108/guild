//! Loading the whole database into an in-memory domain snapshot.

use hangouts_core::Snapshot;

use crate::{Result, Store, StoreError};

impl Store {
    /// Load every table into a [`hangouts_core::Snapshot`], including event
    /// embeddings.
    pub async fn load_snapshot(&self) -> Result<Snapshot> {
        let _ = self;
        Err(StoreError::NotImplemented("load_snapshot"))
    }
}
