//! Native OAuth generation selection while owning the persistent auth store.

use super::covenant_auth_storage::AuthTransaction;
use super::error::RefreshTokenFailedError;
use super::error::RefreshTokenFailedReason;
use super::manager::RefreshTokenError;
use super::manager::persist_tokens;
use super::manager::request_chatgpt_token_refresh;
use super::storage::AuthStorageBackend;
use crate::token_data::TokenData;
use codex_http_client::HttpClient;
use codex_protocol::auth::AuthMode;
use std::sync::Arc;

pub(super) async fn refresh_native_auth(
    transaction: AuthTransaction,
    expected: TokenData,
    client: HttpClient,
) -> Result<(), RefreshTokenError> {
    let unavailable = || {
        RefreshTokenError::Permanent(RefreshTokenFailedError::new(
            RefreshTokenFailedReason::Other,
            "Stored ChatGPT credentials are no longer available for this refresh.".to_string(),
        ))
    };
    // Waiting for ownership remains cancellable without starting a refresh.
    let storage: Arc<dyn AuthStorageBackend> = Arc::new(transaction.lock().await?);
    let current = storage.load()?.ok_or_else(unavailable)?;
    if current.resolved_mode() != AuthMode::Chatgpt {
        return Err(unavailable());
    }
    let stored = current.tokens.as_ref().ok_or_else(unavailable)?;
    if expected.account_id.is_none()
        || stored.account_id != expected.account_id
        || stored.id_token.chatgpt_account_id != expected.id_token.chatgpt_account_id
        || stored.id_token.chatgpt_user_id != expected.id_token.chatgpt_user_id
    {
        return Err(unavailable());
    }
    if stored != &expected {
        // Another owner already committed this account's newer generation.
        // The manager reloads its cache after this guard is released.
        return Ok(());
    }

    // Transfer ownership before polling HTTP: once the authority consumes the
    // token, caller cancellation must not discard its replacement. Dropping
    // this join handle leaves the task running on the same live runtime.
    tokio::spawn(async move {
        let response = request_chatgpt_token_refresh(expected.refresh_token, &client).await?;
        persist_tokens(
            &storage,
            response.id_token,
            response.access_token,
            response.refresh_token,
        )?;
        Ok::<(), RefreshTokenError>(())
    })
    .await
    .map_err(|_| {
        RefreshTokenError::Transient(std::io::Error::other(
            "Native auth refresh task did not complete.",
        ))
    })?
}
