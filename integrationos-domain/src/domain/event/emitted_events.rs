use crate::{ownership::Ownership, Id};

pub struct RefreshTokenExpired {
    pub connection_id: Id,
    pub buildable_id: Ownership,
}

pub enum EmittedEvents {
    RefreshTokenExpired(RefreshTokenExpired),
}
