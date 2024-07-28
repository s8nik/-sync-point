use std::{sync::Arc, time::Duration};

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};

use super::{SyncState, UniqueId, SYNC_TIMEOUT};

pub(super) async fn sync_two_parties(
    Path(unique_id): Path<UniqueId>,
    State(state): State<SyncState>,
) -> impl IntoResponse {
    tracing::debug!("request for synchronization by id: {unique_id}");

    let notify = {
        let mut map = state.lock().await;

        if let Some(notify) = map.remove(&unique_id) {
            tracing::info!("< successful synchronization by id: {unique_id}");
            notify.notify_one();
            return StatusCode::OK;
        }

        Arc::clone(map.entry(unique_id).or_default())
    };

    let duration = Duration::from_secs(*SYNC_TIMEOUT);

    if tokio::time::timeout(duration, notify.notified())
        .await
        .is_err()
    {
        let mut map = state.lock().await;
        map.remove(&unique_id);

        tracing::info!("> synchronization timeout by id: {unique_id}");
        StatusCode::REQUEST_TIMEOUT
    } else {
        tracing::info!("> successful synchronization by id: {unique_id}");
        StatusCode::OK
    }
}

#[cfg(test)]
mod tests {
    use std::{sync::Arc, time::Duration};

    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;

    #[tokio::test]
    async fn sync_two_parties() -> anyhow::Result<()> {
        let state = crate::SyncState::default();

        let first_res = crate::router(Arc::clone(&state)).oneshot(
            Request::builder()
                .method(axum::http::Method::POST)
                .uri("/wait-for-second-party/123")
                .body(Body::empty())?,
        );

        let second_res = crate::router(Arc::clone(&state)).oneshot(
            Request::builder()
                .method(axum::http::Method::POST)
                .uri("/wait-for-second-party/123")
                .body(Body::empty())?,
        );

        let (first_res, second_res) = tokio::try_join!(first_res, second_res)?;

        assert_eq!(first_res.status(), StatusCode::OK);
        assert_eq!(second_res.status(), StatusCode::OK);

        Ok(())
    }

    #[tokio::test]
    async fn sync_failed_timeout() -> anyhow::Result<()> {
        let state = crate::SyncState::default();

        let res = crate::router(Arc::clone(&state)).oneshot(
            Request::builder()
                .method(axum::http::Method::POST)
                .uri("/wait-for-second-party/123")
                .body(Body::empty())?,
        );

        const SYNC_TIMEOUT: u64 = 2;
        let res = tokio::time::timeout(Duration::from_secs(SYNC_TIMEOUT), res).await;

        assert!(res.is_err());

        Ok(())
    }
}
