use std::fs;
use std::path::Path;

use crate::db::open_index;
use crate::error::SkmError;
use crate::store::{init_store_layout, StorePaths};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreState {
    Absent,
    Valid,
    Invalid(String),
}

pub fn inspect_store_path(path: &Path) -> Result<StoreState, SkmError> {
    if path.exists() && !path.is_dir() {
        return Ok(StoreState::Invalid("path is not a directory".to_string()));
    }

    let skm_dir = path.join(".skm");
    if !skm_dir.exists() {
        return Ok(StoreState::Absent);
    }

    if !skm_dir.is_dir() {
        return Ok(StoreState::Invalid(".skm is not a directory".to_string()));
    }

    let store = StorePaths::new(path.to_path_buf());
    match validate_initialized_store(&store) {
        Ok(()) => Ok(StoreState::Valid),
        Err(err) => Ok(StoreState::Invalid(err.to_string())),
    }
}

pub fn validate_initialized_store(store: &StorePaths) -> Result<(), SkmError> {
    let skm_dir = store.skm_dir();
    if !skm_dir.is_dir() {
        return Err(SkmError::InvalidStore {
            path: store.root().to_path_buf(),
            message: format!("missing {}", skm_dir.display()),
        });
    }

    for name in ["profiles", "meta"] {
        let subdir = skm_dir.join(name);
        if !subdir.is_dir() {
            return Err(SkmError::InvalidStore {
                path: store.root().to_path_buf(),
                message: format!("missing {}", subdir.display()),
            });
        }
    }

    open_index(store)?;
    Ok(())
}

pub fn prepare_store(path: &Path) -> Result<StorePaths, SkmError> {
    match inspect_store_path(path)? {
        StoreState::Absent => {
            fs::create_dir_all(path)?;
            let store = StorePaths::new(path.to_path_buf());
            init_store_layout(&store)?;
            Ok(store)
        }
        StoreState::Valid => Ok(StorePaths::new(path.to_path_buf())),
        StoreState::Invalid(message) => Err(SkmError::InvalidStore {
            path: path.to_path_buf(),
            message,
        }),
    }
}
