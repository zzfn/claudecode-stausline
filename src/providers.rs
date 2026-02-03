use chrono::{DateTime, Utc};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use crate::colors;

pub trait Provider {
    fn name(&self) -> &'static str;
    fn matches(&self, base_url: &str) -> bool;
    fn get_parts(&self, base_url: &str, auth_token: &str) -> Vec<String>;
}

/// 质普配额限制信息
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct QuotaLimit {
    #[serde(rename = "type")]
    pub limit_type: String,
    pub percentage: f64,
    #[serde(rename = "currentValue")]
    pub current_value: Option<u64>,
    pub usage: Option<u64>,
}

/// 质普使用情况缓存
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ZhipuUsageCache {
    pub token_limit: Option<QuotaLimit>,
    pub mcp_limit: Option<QuotaLimit>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct YunyiUsageCache {
    pub daily_used: Option<u64>,
    pub daily_quota: Option<u64>,
    pub daily_spent: Option<u64>,
    pub daily_total_spent: Option<u64>,
    pub expires_at: Option<String>,
    pub request_count: Option<u64>,
    pub daily_request_count: Option<u64>,
    pub timestamp: DateTime<Utc>,
}

pub struct ZhipuProvider;

impl ZhipuProvider {
    fn cache_path(&self) -> PathBuf {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".claude").join(".zhipu_cache.json")
    }

    fn read_cache(&self) -> Option<ZhipuUsageCache> {
        let cache_path = self.cache_path();
        let content = fs::read_to_string(cache_path).ok()?;
        let cache: ZhipuUsageCache = serde_json::from_str(&content).ok()?;

        // 检查缓存是否过期（3分钟）
        let now = Utc::now();
        let age = now.signed_duration_since(cache.timestamp);
        if age.num_minutes() < 3 {
            Some(cache)
        } else {
            None
        }
    }

    fn write_cache(&self, cache: &ZhipuUsageCache) {
        let cache_path = self.cache_path();
        if let Ok(json) = serde_json::to_string(cache) {
            let _ = fs::write(cache_path, json);
        }
    }

    fn fetch_usage(&self, base_url: &str, auth_token: &str) -> Option<ZhipuUsageCache> {
        let parsed_url = base_url.parse::<reqwest::Url>().ok()?;
        let base_domain = format!("{}://{}", parsed_url.scheme(), parsed_url.host_str()?);
        let quota_url = format!("{}/api/monitor/usage/quota/limit", base_domain);

        let client = Client::builder()
            .timeout(Duration::from_secs(3))
            .build()
            .ok()?;

        let response = client
            .get(&quota_url)
            .header("Authorization", auth_token)
            .header("Accept-Language", "en-US,en")
            .header("Content-Type", "application/json")
            .send()
            .ok()?;

        if !response.status().is_success() {
            return None;
        }

        #[derive(Deserialize)]
        struct ApiResponse {
            data: ApiData,
        }

        #[derive(Deserialize)]
        struct ApiData {
            limits: Vec<QuotaLimit>,
        }

        let api_response: ApiResponse = response.json().ok()?;

        let mut token_limit = None;
        let mut mcp_limit = None;

        for limit in api_response.data.limits {
            match limit.limit_type.as_str() {
                "TOKENS_LIMIT" => token_limit = Some(limit),
                "TIME_LIMIT" => mcp_limit = Some(limit),
                _ => {}
            }
        }

        let cache = ZhipuUsageCache {
            token_limit,
            mcp_limit,
            timestamp: Utc::now(),
        };

        self.write_cache(&cache);
        Some(cache)
    }

    fn get_usage(&self, base_url: &str, auth_token: &str) -> Option<ZhipuUsageCache> {
        if !self.matches(base_url) {
            return None;
        }

        if let Some(cache) = self.read_cache() {
            return Some(cache);
        }

        self.fetch_usage(base_url, auth_token)
    }
}

impl Provider for ZhipuProvider {
    fn name(&self) -> &'static str {
        "zhipu"
    }

    fn matches(&self, base_url: &str) -> bool {
        base_url.contains("bigmodel.cn") || base_url.contains("z.ai")
    }

    fn get_parts(&self, base_url: &str, auth_token: &str) -> Vec<String> {
        let mut parts = Vec::new();
        let Some(zhipu_usage) = self.get_usage(base_url, auth_token) else {
            return parts;
        };

        if let Some(ref token_limit) = zhipu_usage.token_limit {
            let color = if token_limit.percentage >= 80.0 {
                colors::RED
            } else if token_limit.percentage >= 60.0 {
                colors::YELLOW
            } else {
                colors::GREEN
            };
            parts.push(format!(
                "{}[ZAI] Token(5h):{:.0}%{}",
                color,
                token_limit.percentage,
                colors::RESET
            ));
        }

        if let Some(ref mcp_limit) = zhipu_usage.mcp_limit {
            let color = if mcp_limit.percentage >= 80.0 {
                colors::RED
            } else if mcp_limit.percentage >= 60.0 {
                colors::YELLOW
            } else {
                colors::GREEN
            };
            parts.push(format!(
                "{}[ZAI] MCP(1月):{:.0}%{}",
                color,
                mcp_limit.percentage,
                colors::RESET
            ));
        }

        parts
    }
}

pub struct YunyiProvider;

impl YunyiProvider {
    fn cache_path(&self) -> PathBuf {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".claude").join(".yunyi_cache.json")
    }

    fn read_cache(&self) -> Option<YunyiUsageCache> {
        let cache_path = self.cache_path();
        let content = fs::read_to_string(cache_path).ok()?;
        let cache: YunyiUsageCache = serde_json::from_str(&content).ok()?;

        let now = Utc::now();
        let age = now.signed_duration_since(cache.timestamp);
        if age.num_minutes() < 3 {
            Some(cache)
        } else {
            None
        }
    }

    fn write_cache(&self, cache: &YunyiUsageCache) {
        let cache_path = self.cache_path();
        if let Ok(json) = serde_json::to_string(cache) {
            let _ = fs::write(cache_path, json);
        }
    }

    fn fetch_usage(&self, auth_token: &str) -> Option<YunyiUsageCache> {
        let api_url = "https://yunyi.cfd/user/api/v1/me";
        let bearer = if auth_token.to_ascii_lowercase().starts_with("bearer ") {
            auth_token.to_string()
        } else {
            format!("Bearer {}", auth_token)
        };

        let client = Client::builder()
            .timeout(Duration::from_secs(3))
            .build()
            .ok()?;

        let response = client
            .get(api_url)
            .header("Authorization", bearer)
            .header("Accept", "application/json")
            .header("Accept-Language", "en,zh-CN;q=0.9,zh;q=0.8")
            .send()
            .ok()?;

        if !response.status().is_success() {
            return None;
        }

        #[derive(Deserialize)]
        struct ApiQuota {
            daily_quota: Option<u64>,
            daily_spent: Option<u64>,
            daily_used: Option<u64>,
            daily_total_spent: Option<u64>,
        }

        #[derive(Deserialize)]
        struct ApiUsage {
            request_count: Option<u64>,
            daily_request_count: Option<u64>,
            daily_spent: Option<u64>,
        }

        #[derive(Deserialize)]
        struct ApiTimestamps {
            expires_at: Option<String>,
        }

        #[derive(Deserialize)]
        struct ApiResponse {
            quota: ApiQuota,
            usage: ApiUsage,
            timestamps: ApiTimestamps,
        }

        let api_response: ApiResponse = response.json().ok()?;

        let cache = YunyiUsageCache {
            daily_used: api_response.quota.daily_used,
            daily_quota: api_response.quota.daily_quota,
            daily_spent: api_response.quota.daily_spent.or(api_response.usage.daily_spent),
            daily_total_spent: api_response.quota.daily_total_spent,
            expires_at: api_response.timestamps.expires_at,
            request_count: api_response.usage.request_count,
            daily_request_count: api_response.usage.daily_request_count,
            timestamp: Utc::now(),
        };

        self.write_cache(&cache);
        Some(cache)
    }

    fn get_usage(&self, base_url: &str, auth_token: &str) -> Option<YunyiUsageCache> {
        if !self.matches(base_url) {
            return None;
        }

        if let Some(cache) = self.read_cache() {
            return Some(cache);
        }

        self.fetch_usage(auth_token)
    }
}

impl Provider for YunyiProvider {
    fn name(&self) -> &'static str {
        "yunyi"
    }

    fn matches(&self, base_url: &str) -> bool {
        base_url.contains("yunyi.rdzhvip.com") || base_url.contains("yunyi.cfd")
    }

    fn get_parts(&self, base_url: &str, auth_token: &str) -> Vec<String> {
        let mut parts = Vec::new();
        let Some(usage) = self.get_usage(base_url, auth_token) else {
            return parts;
        };

        if let (Some(quota), Some(total_spent)) = (usage.daily_quota, usage.daily_total_spent) {
            let remaining = quota.saturating_sub(total_spent);
            let remaining_pct = if quota > 0 {
                (remaining as f64 / quota as f64) * 100.0
            } else {
                0.0
            };
            let color = if remaining_pct <= 20.0 {
                colors::RED
            } else if remaining_pct <= 40.0 {
                colors::YELLOW
            } else {
                colors::GREEN
            };
            let remaining_usd = remaining as f64 / 100.0;
            parts.push(format!(
                "{}[YUNYI] Rem:${:.2}{}",
                color,
                remaining_usd,
                colors::RESET
            ));
        }

        if let Some(ref expires_at) = usage.expires_at {
            let formatted = chrono::DateTime::parse_from_rfc3339(expires_at)
                .and_then(|dt| {
                    chrono::FixedOffset::east_opt(8 * 3600)
                        .map(|offset| dt.with_timezone(&offset))
                        .ok_or(chrono::ParseError::NotEnough)
                })
                .map(|dt| dt.format("%m-%d %H:%M").to_string())
                .unwrap_or_else(|_| expires_at.clone());
            parts.push(format!(
                "{}[YUNYI] Exp:{}{}",
                colors::DIM,
                formatted,
                colors::RESET
            ));
        }

        parts
    }
}

pub fn providers() -> Vec<&'static dyn Provider> {
    static ZHIPU_PROVIDER: ZhipuProvider = ZhipuProvider;
    static YUNYI_PROVIDER: YunyiProvider = YunyiProvider;
    vec![&ZHIPU_PROVIDER, &YUNYI_PROVIDER]
}
