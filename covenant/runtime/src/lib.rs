//! Validated Covenant decision requests matching the shared Decide v1 schema.
//!
//! These wire values establish no operating-system authority. Native path,
//! environment, process, and filesystem checks belong to their execution gates.

mod decide;
#[cfg(windows)]
mod decision_reply;
#[cfg(windows)]
mod exec_envelope;
#[cfg(windows)]
mod fanin_binding;
#[cfg(windows)]
mod launch_contract;
mod numbers;
mod patch;
mod sidecar;
mod values;
#[cfg(windows)]
mod windows_environment;
mod wire;

pub use decide::DecideV1;
pub use decide::InvalidDecide;
#[cfg(windows)]
pub use decision_reply::CancellationCompletion;
#[cfg(windows)]
pub use decision_reply::DecisionReply;
#[cfg(windows)]
pub use decision_reply::DecisionReplyError;
#[cfg(windows)]
pub use decision_reply::ProcessCompletion;
#[cfg(windows)]
pub use decision_reply::ProtocolAllow;
#[cfg(windows)]
pub use decision_reply::ReplyCompletion;
#[cfg(windows)]
pub use decision_reply::StdinCompletion;
#[cfg(windows)]
pub use decision_reply::StdoutCompletion;
#[cfg(windows)]
pub use exec_envelope::ExecEvent;
#[cfg(windows)]
pub use exec_envelope::ExecEventError;
#[cfg(windows)]
pub use exec_envelope::FinalExecInput;
#[cfg(windows)]
pub use exec_envelope::NetworkAccess;
#[cfg(windows)]
pub use fanin_binding::FaninBinding;
#[cfg(windows)]
pub use fanin_binding::FaninBindingError;
#[cfg(windows)]
pub use launch_contract::LaunchContract;
#[cfg(windows)]
pub use launch_contract::LaunchContractError;
#[cfg(windows)]
pub use launch_contract::StartupControls;
pub use patch::AncestorIdentity;
pub use patch::IdentityKind;
pub use patch::PatchHunk;
pub use patch::PatchHunkLine;
pub use patch::PatchHunkLineKind;
pub use patch::PatchOperation;
pub use patch::PatchOperationKind;
pub use patch::PatchPermissions;
pub use patch::PatchRequest;
pub use patch::PatchRequestError;
pub use patch::ResolvedIdentity;
pub use patch::encode_patch_request;
pub use sidecar::SidecarClient;
pub use sidecar::SidecarDecision;
pub use sidecar::SidecarError;
#[cfg(windows)]
pub use windows_environment::FrozenWindowsEnvironment;
#[cfg(windows)]
pub use windows_environment::WindowsEnvironmentError;
