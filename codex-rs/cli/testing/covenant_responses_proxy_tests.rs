use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;
use tokio::task::JoinHandle;

use super::{Capture, ConnectionResult, collect_connection};

#[tokio::test]
async fn collect_connection_ignores_cancelled_join_error() {
    let capture = Arc::new(Mutex::new(Capture::default()));
    let mut completed = Vec::new();
    let task: JoinHandle<ConnectionResult> = tokio::spawn(async {
        tokio::time::sleep(Duration::from_millis(50)).await;
        Err(anyhow::anyhow!("owned connection task cancelled in test"))
    });
    task.abort();
    collect_connection(task.await, &mut completed, &capture);
    assert!(
        capture
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .failure
            .is_none()
    );
    assert!(completed.is_empty());
}

#[tokio::test]
async fn collect_connection_records_join_error_other_than_cancelled() {
    let capture = Arc::new(Mutex::new(Capture::default()));
    let mut completed = Vec::new();
    let task: JoinHandle<ConnectionResult> =
        tokio::spawn(async { Err(anyhow::anyhow!("owned connection task failed in test")) });
    collect_connection(task.await, &mut completed, &capture);
    assert_eq!(
        capture
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .failure,
        Some("owned connection refused")
    );
    assert!(completed.is_empty());
}
