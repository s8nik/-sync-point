use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};

use super::{SyncState, UniqueId};

pub(super) async fn sync_two_parties(
    Path(unique_id): Path<UniqueId>,
    State(state): State<SyncState>,
) -> impl IntoResponse {
    tracing::debug!("request for synchronization by id: {unique_id}");

    let notify = {
        let mut map = state.map.lock().await;

        if let Some(notify) = map.remove(&unique_id) {
            tracing::info!("< successful synchronization by id: {unique_id}");
            notify.notify_one();
            return StatusCode::OK;
        }

        Arc::clone(map.entry(unique_id).or_default())
    };

    if tokio::time::timeout(state.timeout, notify.notified())
        .await
        .is_err()
    {
        let mut map = state.map.lock().await;
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
    use std::time::Duration;

    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;

    const SYNC_TIMEOUT: u64 = 1;

    #[tokio::test]
    async fn sync_two_parties() -> anyhow::Result<()> {
        let state = crate::SyncState {
            map: crate::SyncMap::default(),
            timeout: Duration::from_secs(SYNC_TIMEOUT),
        };

        let first_res = crate::router(state.clone()).oneshot(
            Request::builder()
                .method(axum::http::Method::POST)
                .uri("/wait-for-second-party/123")
                .body(Body::empty())?,
        );

        let second_res = crate::router(state.clone()).oneshot(
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
        let state = crate::SyncState {
            map: crate::SyncMap::default(),
            timeout: Duration::from_secs(SYNC_TIMEOUT),
        };

        let res = crate::router(state.clone())
            .oneshot(
                Request::builder()
                    .method(axum::http::Method::POST)
                    .uri("/wait-for-second-party/123")
                    .body(Body::empty())?,
            )
            .await?;

        assert_eq!(res.status(), StatusCode::REQUEST_TIMEOUT);

        Ok(())
    }
}
