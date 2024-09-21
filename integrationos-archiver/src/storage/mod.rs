pub mod google_cloud;

use crate::config::ArchiverConfig;
use anyhow::Result;
use integrationos_domain::Unit;
use std::{future::Future, ops::Deref, path::Path};
use strum::{AsRefStr, EnumString};

#[derive(Debug, Clone, PartialEq, Eq, EnumString, AsRefStr)]
#[strum(serialize_all = "kebab-case")]
pub enum StorageProvider {
    GoogleCloud,
    // TODO: Implement LocalStorage
}

#[derive(Debug)]
pub struct Chunk {
    data: Vec<u8>,
    first_byte: u64,
    last_byte: u64,
}

impl Chunk {
    fn first_byte(&self) -> u64 {
        self.first_byte
    }

    fn last_byte(&self) -> u64 {
        self.last_byte
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Extension {
    Bson,
    Metadata,
}

impl AsRef<str> for Extension {
    fn as_ref(&self) -> &str {
        match self {
            Extension::Bson => "bson.gz",
            Extension::Metadata => "metadata.json.gz",
        }
    }
}

impl Deref for Extension {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

pub trait Storage {
    fn upload_file(
        &self,
        base_path: &Path,
        extension: &Extension,
        config: &ArchiverConfig,
        suffix: String,
    ) -> impl Future<Output = Result<Unit>>;
}
