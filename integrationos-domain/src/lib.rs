pub mod algebra;
pub mod domain;
pub mod service;

pub use crate::algebra::*;
pub use crate::domain::*;
pub use crate::service::*;

pub mod prelude {
    pub use crate::algebra::*;
    pub use crate::domain::*;
    pub use crate::service::*;
}
