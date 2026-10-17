pub mod hianime;
pub mod lookmovie;
pub mod vixsrc;

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

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
    pub quality: Option<String>,
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
/// If the primary provider (VixSrc) fails or 404s, it smoothly attempts secondary providers
/// (HiAnime for anime, LookMovie for movies/shows) so playback never abruptly fails for the user.
pub struct ProviderManager {
    providers: Vec<Arc<dyn Provider>>,
}

impl ProviderManager {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            providers: vec![
                Arc::new(vixsrc::VixSrcProvider::new()),
                Arc::new(hianime::HiAnimeProvider::new()),
                Arc::new(lookmovie::LookMovieProvider::new()),
            ],
        }
    }

    #[allow(dead_code)]
    pub fn with_providers(providers: Vec<Arc<dyn Provider>>) -> Self {
        Self { providers }
    }

    pub fn with_preference(preferred: &str) -> Self {
        let vix = Arc::new(vixsrc::VixSrcProvider::new());
        let hianime = Arc::new(hianime::HiAnimeProvider::new());
        let look = Arc::new(lookmovie::LookMovieProvider::new());

        let providers: Vec<Arc<dyn Provider>> = match preferred.to_lowercase().as_str() {
            "hianime" => vec![hianime, vix, look],
            "lookmovie" => vec![look, vix, hianime],
            "vixsrc_only" => vec![vix],
            "hianime_only" => vec![hianime],
            "lookmovie_only" => vec![look],
            _ => vec![vix, hianime, look],
        };

        Self { providers }
    }

    /// Resolves using a specific provider by identifier ("auto", "vixsrc", "hianime", "lookmovie").
    /// If "auto", performs full fallback across configured providers.
    #[allow(dead_code)]
    pub async fn resolve_with_choice(
        &self,
        choice: &str,
        req: &MediaRequest,
    ) -> Result<StreamCandidate, String> {
        self.resolve_with_choice_and_quality(choice, req, None).await
    }

    /// Resolves using a specific provider and applies the requested quality preference.
    pub async fn resolve_with_choice_and_quality(
        &self,
        choice: &str,
        req: &MediaRequest,
        quality: Option<&str>,
    ) -> Result<StreamCandidate, String> {
        let key = choice.to_lowercase();
        let mut candidate = if key.contains("hianime") {
            let mut res = None;
            for provider in &self.providers {
                if provider.name().to_lowercase().contains("hianime") {
                    res = Some(provider.resolve(req).await?);
                    break;
                }
            }
            let mut cand = if let Some(c) = res {
                c
            } else {
                let hi = hianime::HiAnimeProvider::new();
                hi.resolve_with_quality(req, quality, None).await?
            };
            if let Some(q) = quality {
                cand.quality = Some(q.to_string());
            }
            cand
        } else if key.contains("vixsrc") {
            let mut res = None;
            for provider in &self.providers {
                if provider.name().to_lowercase().contains("vixsrc") {
                    res = Some(provider.resolve(req).await?);
                    break;
                }
            }
            let mut cand = if let Some(c) = res {
                c
            } else {
                let vix = vixsrc::VixSrcProvider::new();
                vix.resolve(req).await?
            };
            if let Some(q) = quality {
                cand.quality = Some(q.to_string());
            }
            cand
        } else if key.contains("lookmovie") {
            let mut res = None;
            for provider in &self.providers {
                if provider.name().to_lowercase().contains("lookmovie") {
                    res = Some(provider.resolve(req).await?);
                    break;
                }
            }
            let mut cand = if let Some(c) = res {
                c
            } else {
                let look = lookmovie::LookMovieProvider::new();
                look.resolve_with_quality(req, quality).await?
            };
            if cand.quality.is_none() {
                if let Some(q) = quality {
                    cand.quality = Some(q.to_string());
                }
            }
            cand
        } else {
            let mut cand = self.resolve_with_fallback(req).await?;
            if let Some(q) = quality {
                if cand.quality.is_none() {
                    cand.quality = Some(q.to_string());
                }
            }
            cand
        };

        // If candidate has no external subtitles, opportunistically augment from LookMovie with a short timeout
        if candidate.subtitles.is_empty()
            && !candidate.provider_name.to_lowercase().contains("lookmovie")
            && !candidate.provider_name.to_lowercase().contains("hianime")
        {
            for provider in &self.providers {
                if provider.name().to_lowercase().contains("lookmovie") {
                    if let Ok(Ok(look_cand)) =
                        tokio::time::timeout(Duration::from_millis(2500), provider.resolve(req))
                            .await
                    {
                        if !look_cand.subtitles.is_empty() {
                            candidate.subtitles = look_cand.subtitles;
                        }
                    }
                    break;
                }
            }
        }

        Ok(candidate)
    }

    #[allow(dead_code)]
    pub fn providers(&self) -> &[Arc<dyn Provider>] {
        &self.providers
    }

    /// Tries each provider in sequential priority order until one succeeds.
    /// Returns the first working StreamCandidate, or an aggregate error if all fail.
    pub async fn resolve_with_fallback(
        &self,
        req: &MediaRequest,
    ) -> Result<StreamCandidate, String> {
        let mut failures = Vec::new();

        for provider in &self.providers {
            match provider.resolve(req).await {
                Ok(mut candidate) => {
                    if candidate.subtitles.is_empty()
                        && !candidate.provider_name.to_lowercase().contains("lookmovie")
                        && !candidate.provider_name.to_lowercase().contains("hianime")
                    {
                        for p in &self.providers {
                            if p.name().to_lowercase().contains("lookmovie") {
                                if let Ok(Ok(look_cand)) = tokio::time::timeout(
                                    Duration::from_millis(2500),
                                    p.resolve(req),
                                )
                                .await
                                {
                                    if !look_cand.subtitles.is_empty() {
                                        candidate.subtitles = look_cand.subtitles;
                                    }
                                }
                                break;
                            }
                        }
                    }
                    return Ok(candidate);
                }
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
                    quality: None,
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
            Arc::new(MockProvider {
                name: "MockBroken",
                succeed: false,
            }),
            Arc::new(MockProvider {
                name: "MockWorking",
                succeed: true,
            }),
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
            Arc::new(MockProvider {
                name: "Broken1",
                succeed: false,
            }),
            Arc::new(MockProvider {
                name: "Broken2",
                succeed: false,
            }),
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
        assert!(
            res.is_ok(),
            "LookMovie should resolve Fight Club: {:?}",
            res.err()
        );
        let candidate = res.unwrap();
        assert!(candidate.stream_url.contains(".m3u8"));
        assert_eq!(candidate.provider_name, "LookMovie (Fallback CDN)");
    }

    #[tokio::test]
    async fn test_lookmovie_live_resolve_tv_show() {
        let provider = lookmovie::LookMovieProvider::new();
        let req = MediaRequest {
            tmdb_id: 4951,
            media_type: "tv".to_string(),
            season: Some(1),
            episode: Some(1),
            title: "Steven Universe".to_string(),
            year: Some("2013".to_string()),
        };

        let res = provider.resolve(&req).await;
        println!("LookMovie TV show result: {:?}", res);
        assert!(
            res.is_ok(),
            "LookMovie should resolve Steven Universe: {:?}",
            res.err()
        );
    }

    #[tokio::test]
    async fn test_provider_manager_resolve_with_choice() {
        let mgr = ProviderManager::with_providers(vec![
            Arc::new(MockProvider {
                name: "vixsrc_server",
                succeed: true,
            }),
            Arc::new(MockProvider {
                name: "lookmovie_server",
                succeed: true,
            }),
        ]);

        let req = MediaRequest {
            tmdb_id: 550,
            media_type: "movie".to_string(),
            season: None,
            episode: None,
            title: "Fight Club".to_string(),
            year: Some("1999".to_string()),
        };

        let vix_res = mgr.resolve_with_choice("vixsrc", &req).await.unwrap();
        assert_eq!(vix_res.provider_name, "vixsrc_server");

        let look_res = mgr.resolve_with_choice("lookmovie", &req).await.unwrap();
        assert_eq!(look_res.provider_name, "lookmovie_server");

        let auto_res = mgr.resolve_with_choice("auto", &req).await.unwrap();
        assert_eq!(auto_res.provider_name, "vixsrc_server");
    }

    #[tokio::test]
    async fn test_hianime_live_resolve_anime() {
        let provider = hianime::HiAnimeProvider::new();
        let req = MediaRequest {
            tmdb_id: 1111,
            media_type: "tv".to_string(),
            season: Some(1),
            episode: Some(1),
            title: "Uma Musume: Cinderella Gray".to_string(),
            year: Some("2025".to_string()),
        };

        let res = provider.resolve(&req).await;
        println!("HiAnime Cinderella Gray result: {:?}", res);
        assert!(
            res.is_ok(),
            "HiAnime should resolve Cinderella Gray: {:?}",
            res.err()
        );
        let candidate = res.unwrap();
        assert!(candidate.stream_url.contains(".m3u8"));
        assert!(candidate.provider_name.contains("HiAnime"));
    }
}
