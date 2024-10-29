pub mod completed;
pub mod dumped;
pub mod failed;
pub mod finished;
pub mod started;
pub mod uploaded;

use completed::Completed;
use dumped::Dumped;
use failed::Failed;
use finished::Finished;
use integrationos_domain::Id;
use serde::{Deserialize, Serialize};
use started::Started;
use uploaded::Uploaded;

pub trait EventMetadata {
    fn reference(&self) -> Id;
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum Event {
    /// Archive process started event. Emitted when the archive process is started.
    Started(Started),
    /// Archive process dumped event. Emitted when mongodump finishes dumping the database.
    Dumped(Dumped),
    /// Archive process failed event. Emitted when the archive process fails in some way.
    Failed(Failed),
    /// Archive process uploaded event. Emitted after the selected storage provider uploads any file (by default, the archive file and metadata file).
    Uploaded(Uploaded),
    /// Archive process completed event. Emitted when all dumped files are uploaded.
    Completed(Completed),
    /// Archive process finished event. Emitted when the archive process is finished.
    Finished(Finished),
}
