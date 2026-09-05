use thiserror::Error;

/// Error returned while executing a model-visible tool invocation.
#[derive(Debug, Error, PartialEq)]
pub enum FunctionCallError {
    #[error("CovenantDenied {{ code: unknown_tool }}")]
    CovenantDenied,
    #[error("{0}")]
    RespondToModel(String),
    #[error("Fatal error: {0}")]
    Fatal(String),
}
