//! Validated Covenant decision requests matching the shared Decide v1 schema.
//!
//! These wire values establish no operating-system authority. Native path,
//! environment, process, and filesystem checks belong to their execution gates.

mod decide;
mod numbers;
mod values;
mod wire;

pub use decide::DecideV1;
pub use decide::InvalidDecide;
