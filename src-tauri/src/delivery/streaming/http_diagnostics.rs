//! Aggregate request-handler diagnostics without recording media paths or tokens.

use axum::{body::Body, extract::State, http::Request, middleware::Next, response::Response};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

const ROUTES: [&str; 7] = [
    "health", "probe", "stream", "playlist", "segment", "hls_live", "other",
];

#[derive(Default)]
pub(super) struct ActiveHandlers {
    counts: [AtomicUsize; ROUTES.len()],
}

impl ActiveHandlers {
    #[cfg(test)]
    pub(super) fn count(&self, route: usize) -> usize {
        self.counts[route].load(Ordering::Relaxed)
    }

    fn begin(self: &Arc<Self>, path: &str) -> ActiveHandler {
        let route = match path.split('/').nth(1) {
            Some("health") => 0,
            Some("probe") => 1,
            Some("stream") => 2,
            Some("playlist") => 3,
            Some("segment") => 4,
            Some("hls-live") => 5,
            _ => 6,
        };
        self.counts[route].fetch_add(1, Ordering::Relaxed);
        ActiveHandler {
            handlers: self.clone(),
            route,
        }
    }

    pub(super) fn log_active(&self) {
        let mut total = 0;
        for (route, count) in ROUTES.iter().zip(&self.counts) {
            let active = count.load(Ordering::Relaxed);
            total += active;
            if active > 0 {
                tracing::info!(
                    route,
                    active,
                    "Streaming HTTP handlers still active during shutdown"
                );
            }
        }
        tracing::info!(
            active_handlers = total,
            "Streaming HTTP handler snapshot; response bodies and connections are not tracked"
        );
    }
}

struct ActiveHandler {
    handlers: Arc<ActiveHandlers>,
    route: usize,
}

impl Drop for ActiveHandler {
    fn drop(&mut self) {
        self.handlers.counts[self.route].fetch_sub(1, Ordering::Relaxed);
    }
}

pub(super) async fn track_handler(
    State(handlers): State<Arc<ActiveHandlers>>,
    request: Request<Body>,
    next: Next,
) -> Response {
    let _active = handlers.begin(request.uri().path());
    next.run(request).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tracks_and_releases_only_route_categories() {
        let handlers = Arc::new(ActiveHandlers::default());
        let first = handlers.begin("/hls-live/private/index.m3u8");
        let second = handlers.begin("/hls-live/another/segment.ts");
        assert_eq!(handlers.counts[5].load(Ordering::Relaxed), 2);
        drop(first);
        assert_eq!(handlers.counts[5].load(Ordering::Relaxed), 1);
        drop(second);
        assert_eq!(handlers.counts[5].load(Ordering::Relaxed), 0);
    }
}
