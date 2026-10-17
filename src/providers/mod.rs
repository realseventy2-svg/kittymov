pub mod lookmovie;
pub mod vixsrc;

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct MediaRequest {
    pub tmdb_id: i64,
    pub media_type: String, // "movie" or "tv"
    pub season: Option<i32>,
    pub episode: Option<i32>,
    pub title: String,
    pub year: Option<String>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct StreamCandidate {
    pub stream_url: String,
    pub provider_name: String,
    pub headers: HashMap<String, String>,
    pub subtitles: Vec<String>,
    pub expires_at: u64,
    pub embed_url: Option<String>,
    pub request: MediaRequest,
    pub actual_title: Option<String>,
}

impl StreamCandidate {
    pub fn is_expired_or_expiring_soon(&self) -> bool {
        if self.expires_at == 0 {
            return false;
        }
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        // If it expires in less than 90 seconds, consider it expiring soon
        now + 90 >= self.expires_at
    }
}

#[async_trait]
pub trait Provider: Send + Sync {
    fn name(&self) -> &'static str;
    async fn resolve(&self, req: &MediaRequest) -> Result<StreamCandidate, String>;
    async fn refresh_token(&self, candidate: &mut StreamCandidate) -> Result<(), String>;
}

/// ProviderManager orchestrates multi-provider stream resolution with automatic fallback.
/// If the primary provider (VixSrc) fails or 404s, it smoothly attempts the secondary
/// provider (LookMovie) so playback never abruptly fails for the user.
pub struct ProviderManager {
    providers: Vec<Arc<dyn Provider>>,
}

impl ProviderManager {
    pub fn new() -> Self {
        Self {
            providers: vec![
                Arc::new(vixsrc::VixSrcProvider::new()),
                Arc::new(lookmovie::LookMovieProvider::new()),
            ],
        }
    }

    #[allow(dead_code)]
    pub fn with_providers(providers: Vec<Arc<dyn Provider>>) -> Self {
        Self { providers }
    }

    #[allow(dead_code)]
    pub fn providers(&self) -> &[Arc<dyn Provider>] {
        &self.providers
    }

    /// Tries each provider in sequential priority order until one succeeds.
    /// Returns the first working StreamCandidate, or an aggregate error if all fail.
    pub async fn resolve_with_fallback(&self, req: &MediaRequest) -> Result<StreamCandidate, String> {
        let mut failures = Vec::new();

        for provider in &self.providers {
            match provider.resolve(req).await {
                Ok(candidate) => return Ok(candidate),
                Err(err) => {
                    failures.push(format!("{}: {}", provider.name(), err));
                }
            }
        }

        Err(format!(
            "All providers failed to resolve stream:\n  • {}",
            failures.join("\n  • ")
        ))
    }

    /// Refreshes token / link if expired or expiring soon, routing to the originating provider.
    pub async fn refresh_candidate(&self, candidate: &mut StreamCandidate) -> Result<(), String> {
        for provider in &self.providers {
            if provider.name() == candidate.provider_name {
                return provider.refresh_token(candidate).await;
            }
        }
        // If originating provider isn't recognized, re-resolve with fallback
        let fresh = self.resolve_with_fallback(&candidate.request).await?;
        *candidate = fresh;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockProvider {
        name: &'static str,
        succeed: bool,
    }

    #[async_trait]
    impl Provider for MockProvider {
        fn name(&self) -> &'static str {
            self.name
        }

        async fn resolve(&self, req: &MediaRequest) -> Result<StreamCandidate, String> {
            if self.succeed {
                Ok(StreamCandidate {
                    stream_url: "https://mock.cdn/master.m3u8".to_string(),
                    provider_name: self.name.to_string(),
                    headers: HashMap::new(),
                    subtitles: Vec::new(),
                    expires_at: 0,
                    embed_url: None,
                    request: req.clone(),
                    actual_title: Some("Mock Title".to_string()),
                })
            } else {
                Err("Simulated 404 / connection timeout".to_string())
            }
        }

        async fn refresh_token(&self, _candidate: &mut StreamCandidate) -> Result<(), String> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_provider_manager_fallback_success() {
        let mgr = ProviderManager::with_providers(vec![
            Arc::new(MockProvider { name: "MockBroken", succeed: false }),
            Arc::new(MockProvider { name: "MockWorking", succeed: true }),
        ]);

        let req = MediaRequest {
            tmdb_id: 550,
            media_type: "movie".to_string(),
            season: None,
            episode: None,
            title: "Fight Club".to_string(),
            year: Some("1999".to_string()),
        };

        let result = mgr.resolve_with_fallback(&req).await;
        assert!(result.is_ok(), "Expected fallback provider to succeed");
        let candidate = result.unwrap();
        assert_eq!(candidate.provider_name, "MockWorking");
        assert_eq!(candidate.stream_url, "https://mock.cdn/master.m3u8");
    }

    #[tokio::test]
    async fn test_provider_manager_all_failed() {
        let mgr = ProviderManager::with_providers(vec![
            Arc::new(MockProvider { name: "Broken1", succeed: false }),
            Arc::new(MockProvider { name: "Broken2", succeed: false }),
        ]);

        let req = MediaRequest {
            tmdb_id: 123,
            media_type: "movie".to_string(),
            season: None,
            episode: None,
            title: "Nonexistent".to_string(),
            year: None,
        };

        let result = mgr.resolve_with_fallback(&req).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("Broken1"));
        assert!(err.contains("Broken2"));
    }

    #[tokio::test]
    async fn test_lookmovie_live_resolve_movie() {
        let provider = lookmovie::LookMovieProvider::new();
        let req = MediaRequest {
            tmdb_id: 550,
            media_type: "movie".to_string(),
            season: None,
            episode: None,
            title: "Fight Club".to_string(),
            year: Some("1999".to_string()),
        };

        let res = provider.resolve(&req).await;
        assert!(res.is_ok(), "LookMovie should resolve Fight Club: {:?}", res.err());
        let candidate = res.unwrap();
        assert!(candidate.stream_url.contains(".m3u8"));
        assert_eq!(candidate.provider_name, "LookMovie (Fallback CDN)");
    }
}
