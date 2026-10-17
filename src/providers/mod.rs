pub mod vixsrc;

use async_trait::async_trait;
use std::collections::HashMap;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct MediaRequest {
    pub tmdb_id: i64,
    pub media_type: String, // "movie" or "tv"
    pub season: Option<i32>,
    pub episode: Option<i32>,
    pub title: String,
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
