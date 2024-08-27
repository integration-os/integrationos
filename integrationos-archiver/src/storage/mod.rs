pub mod google_cloud;

use crate::{config::ArchiverConfig, event::Event};
use anyhow::Result;
use chrono::NaiveDate;
use integrationos_domain::Unit;
use std::{
    future::Future,
    ops::Deref,
    path::{Path, PathBuf},
};

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

#[derive(Debug)]
struct ArchiveName {
    date: NaiveDate,
    name: String,
    extension: Extension,
}

impl ArchiveName {
    fn name(&self) -> String {
        format!(
            "{}-{}.{}",
            self.date.format("%Y-%m-%d"),
            self.name,
            self.extension.as_ref()
        )
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Extension {
    Bson,
    Metadata,
}

impl Extension {
    /// Returns the file extension for the given extension with the leading dot
    fn with_leading_dot(self) -> String {
        ".".to_owned() + self.as_ref()
    }
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
    ) -> impl Future<Output = Result<Unit>>;

    fn download_file(
        &self,
        config: &ArchiverConfig,
        event: &Event,
        extension: &Extension,
    ) -> impl Future<Output = Result<PathBuf>>;
}
