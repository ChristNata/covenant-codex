//! Validated Covenant decision requests matching the shared Decide v1 schema.
//!
//! These wire values establish no operating-system authority. Native path,
//! environment, process, and filesystem checks belong to their execution gates.

mod decide;
#[cfg(windows)]
mod exec_envelope;
#[cfg(windows)]
mod launch_contract;
mod numbers;
mod values;
#[cfg(windows)]
mod windows_environment;
mod wire;

pub use decide::DecideV1;
pub use decide::InvalidDecide;
#[cfg(windows)]
pub use exec_envelope::ExecEvent;
#[cfg(windows)]
pub use exec_envelope::ExecEventError;
#[cfg(windows)]
pub use exec_envelope::FinalExecInput;
#[cfg(windows)]
pub use exec_envelope::NetworkAccess;
#[cfg(windows)]
pub use launch_contract::LaunchContract;
#[cfg(windows)]
pub use launch_contract::LaunchContractError;
#[cfg(windows)]
pub use launch_contract::StartupControls;
#[cfg(windows)]
pub use windows_environment::FrozenWindowsEnvironment;
#[cfg(windows)]
pub use windows_environment::WindowsEnvironmentError;
