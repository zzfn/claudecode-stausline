use serde::{Deserialize, Serialize};
use std::io::{self, Read};
use std::process::Command;
use std::fs;
use std::path::PathBuf;
use chrono::{DateTime, Utc};
use reqwest::blocking::Client;
use std::time::Duration;

/// 模型信息
#[derive(Debug, Deserialize, Default)]
pub struct Model {
    pub id: Option<String>,
    pub display_name: Option<String>,
}

/// 工作区信息
#[derive(Debug, Deserialize, Default)]
pub struct Workspace {
    pub current_dir: Option<String>,
    pub project_dir: Option<String>,
}

/// 成本统计
#[derive(Debug, Deserialize, Default)]
pub struct Cost {
    pub total_cost_usd: Option<f64>,
    pub total_duration_ms: Option<u64>,
    pub total_api_duration_ms: Option<u64>,
    pub total_lines_added: Option<u64>,
    pub total_lines_removed: Option<u64>,
}

/// 当前使用情况
#[derive(Debug, Deserialize, Default)]
pub struct CurrentUsage {
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub cache_creation_input_tokens: Option<u64>,
    pub cache_read_input_tokens: Option<u64>,
}

/// 上下文窗口信息
#[derive(Debug, Deserialize, Default)]
pub struct ContextWindow {
    pub total_input_tokens: Option<u64>,
    pub total_output_tokens: Option<u64>,
    pub context_window_size: Option<u64>,
    pub used_percentage: Option<f64>,
    pub remaining_percentage: Option<f64>,
    pub current_usage: Option<CurrentUsage>,
}

/// 输出样式
#[derive(Debug, Deserialize, Default)]
pub struct OutputStyle {
    pub name: Option<String>,
}

/// Claude Code Statusline 输入数据结构
#[derive(Debug, Deserialize, Default)]
pub struct StatusInput {
    pub hook_event_name: Option<String>,
    pub session_id: Option<String>,
    pub transcript_path: Option<String>,
    pub cwd: Option<String>,
    pub version: Option<String>,
    #[serde(default)]
    pub model: Model,
    #[serde(default)]
    pub workspace: Workspace,
    #[serde(default)]
    pub cost: Cost,
    #[serde(default)]
    pub context_window: ContextWindow,
    #[serde(default)]
    pub output_style: OutputStyle,
}

/// ANSI 颜色代码
mod colors {
    pub const RESET: &str = "\x1b[0m";
    pub const BOLD: &str = "\x1b[1m";
    pub const DIM: &str = "\x1b[2m";

    pub const RED: &str = "\x1b[31m";
    pub const GREEN: &str = "\x1b[32m";
    pub const YELLOW: &str = "\x1b[33m";
    pub const BLUE: &str = "\x1b[34m";
    pub const MAGENTA: &str = "\x1b[35m";
    pub const CYAN: &str = "\x1b[36m";
}

/// 根据使用百分比返回对应颜色
fn get_context_color(percentage: f64) -> &'static str {
    if percentage >= 80.0 {
        colors::RED
    } else if percentage >= 60.0 {
        colors::YELLOW
    } else {
        colors::GREEN
    }
}

/// 格式化成本显示
fn format_cost(cost: f64) -> String {
    if cost < 0.01 {
        format!("{:.4}", cost)
    } else if cost < 1.0 {
        format!("{:.3}", cost)
    } else {
        format!("{:.2}", cost)
    }
}

/// 从路径中提取目录名
fn get_dir_name(path: &str) -> &str {
    std::path::Path::new(path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(path)
}

/// 获取当前 git 分支名
fn get_git_branch(cwd: Option<&str>) -> Option<String> {
    let cwd = cwd?; // 如果没有工作目录,直接返回 None

    let output = Command::new("git")
        .args(&["branch", "--show-current"])
        .current_dir(cwd)
        .output()
        .ok()?;

    if output.status.success() {
        let branch = String::from_utf8(output.stdout).ok()?;
        let branch = branch.trim();
        if !branch.is_empty() {
            return Some(branch.to_string());
        }
    }
    None
}

/// 获取未提交的文件数量
fn get_uncommitted_files(cwd: Option<&str>) -> Option<usize> {
    let cwd = cwd?;

    let output = Command::new("git")
        .args(&["status", "--porcelain"])
        .current_dir(cwd)
        .output()
        .ok()?;

    if output.status.success() {
        let status = String::from_utf8(output.stdout).ok()?;
        let count = status.lines().filter(|line| !line.is_empty()).count();
        if count > 0 {
            return Some(count);
        }
    }
    None
}

/// 格式化会话时长
fn format_duration(ms: u64) -> String {
    let seconds = ms / 1000;
    let minutes = seconds / 60;
    let hours = minutes / 60;

    if hours > 0 {
        format!("{}h{}m", hours, minutes % 60)
    } else if minutes > 0 {
        format!("{}m", minutes)
    } else {
        format!("{}s", seconds)
    }
}

/// 计算缓存命中率
fn calculate_cache_hit_rate(usage: &CurrentUsage) -> Option<f64> {
    let cache_read = usage.cache_read_input_tokens?;
    let total_input = usage.input_tokens?;

    if total_input == 0 {
        return None;
    }

    let hit_rate = (cache_read as f64 / total_input as f64) * 100.0;
    Some(hit_rate)
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

/// 获取缓存文件路径
fn get_cache_path() -> PathBuf {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".claude").join(".zhipu_cache.json")
}

/// 读取缓存
fn read_cache() -> Option<ZhipuUsageCache> {
    let cache_path = get_cache_path();
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

/// 写入缓存
fn write_cache(cache: &ZhipuUsageCache) {
    let cache_path = get_cache_path();
    if let Ok(json) = serde_json::to_string(cache) {
        let _ = fs::write(cache_path, json);
    }
}

/// 从质普 API 获取使用情况
fn fetch_zhipu_usage(base_url: &str, auth_token: &str) -> Option<ZhipuUsageCache> {
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

    write_cache(&cache);
    Some(cache)
}

/// Claude Code 配置文件结构
#[derive(Debug, Deserialize)]
struct ClaudeConfig {
    #[serde(rename = "baseURL")]
    base_url: Option<String>,
    #[serde(rename = "authToken")]
    auth_token: Option<String>,
}

/// 从 Claude Code 配置文件读取配置
fn read_claude_config() -> Option<(String, String)> {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .ok()?;
    let config_path = PathBuf::from(home).join(".claude").join("settings.json");

    let content = fs::read_to_string(config_path).ok()?;
    let config: ClaudeConfig = serde_json::from_str(&content).ok()?;

    let base_url = config.base_url?;
    let auth_token = config.auth_token?;

    Some((base_url, auth_token))
}

/// 获取质普使用情况（带缓存）
fn get_zhipu_usage() -> Option<ZhipuUsageCache> {
    // 先从配置文件或环境变量获取 base_url，检查是否是质普域名
    let (base_url, auth_token) = read_claude_config()
        .or_else(|| {
            let base_url = std::env::var("ANTHROPIC_BASE_URL").ok()?;
            let auth_token = std::env::var("ANTHROPIC_AUTH_TOKEN").ok()?;
            Some((base_url, auth_token))
        })?;

    // 检查是否是质普域名
    if !base_url.contains("bigmodel.cn") && !base_url.contains("z.ai") {
        return None;
    }

    // 确认是质普域名后，再尝试读取缓存
    if let Some(cache) = read_cache() {
        return Some(cache);
    }

    fetch_zhipu_usage(&base_url, &auth_token)
}

/// 构建 statusline 输出
fn build_statusline(input: &StatusInput) -> String {
    let mut parts = Vec::new();

    // 模型名称
    if let Some(ref name) = input.model.display_name {
        parts.push(format!(
            "{}{}[{}]{}",
            colors::BOLD,
            colors::MAGENTA,
            name,
            colors::RESET
        ));
    }

    // 当前目录
    if let Some(ref dir) = input.workspace.current_dir {
        let dir_name = get_dir_name(dir);
        parts.push(format!(
            "{}{}{}",
            colors::CYAN,
            dir_name,
            colors::RESET
        ));
    }

    // Git 分支
    if let Some(branch) = get_git_branch(input.workspace.current_dir.as_deref()) {
        parts.push(format!(
            "{}{}{}",
            colors::BLUE,
            branch,
            colors::RESET
        ));
    }

    // 上下文使用率
    let percentage = input.context_window.used_percentage.or_else(|| {
        // 如果没有 used_percentage，尝试从其他字段计算
        let total_in = input.context_window.total_input_tokens?;
        let total_out = input.context_window.total_output_tokens?;
        let window_size = input.context_window.context_window_size?;
        if window_size > 0 {
            Some(((total_in + total_out) as f64 / window_size as f64) * 100.0)
        } else {
            None
        }
    });

    if let Some(percentage) = percentage {
        let color = get_context_color(percentage);
        parts.push(format!(
            "{}ctx:{:.0}%{}",
            color,
            percentage,
            colors::RESET
        ));
    }

    // Token 统计
    if let Some(ref usage) = input.context_window.current_usage {
        if let Some(input_tokens) = usage.input_tokens {
            let formatted = if input_tokens >= 1000 {
                format!("{:.1}k", input_tokens as f64 / 1000.0)
            } else {
                format!("{}", input_tokens)
            };
            parts.push(format!(
                "{}in:{}{}",
                colors::DIM,
                formatted,
                colors::RESET
            ));
        }

        // 缓存命中率
        if let Some(hit_rate) = calculate_cache_hit_rate(usage) {
            if hit_rate > 0.0 {
                let color = if hit_rate >= 80.0 {
                    colors::GREEN
                } else if hit_rate >= 50.0 {
                    colors::YELLOW
                } else {
                    colors::RED
                };
                parts.push(format!(
                    "{}cache:{:.0}%{}",
                    color,
                    hit_rate,
                    colors::RESET
                ));
            }
        }
    }

    // 质普使用情况（放在最后）
    if let Some(zhipu_usage) = get_zhipu_usage() {
        // Token 使用量（5小时）
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

        // MCP 使用量（1个月）
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
    }

    parts.join(" │ ")
}

fn main() {
    // 从 stdin 读取 JSON 输入
    let mut input_str = String::new();
    if io::stdin().read_to_string(&mut input_str).is_err() {
        println!("Error reading stdin");
        return;
    }

    // 解析 JSON
    let input: StatusInput = match serde_json::from_str(&input_str) {
        Ok(data) => data,
        Err(_) => {
            println!("Error parsing JSON");
            return;
        }
    };

    // 输出 statusline
    println!("{}", build_statusline(&input));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_dir_name() {
        assert_eq!(get_dir_name("/Users/test/project"), "project");
        assert_eq!(get_dir_name("project"), "project");
        assert_eq!(get_dir_name("/"), "");
    }

    #[test]
    fn test_format_cost() {
        assert_eq!(format_cost(0.0001), "0.0001");
        assert_eq!(format_cost(0.123), "0.123");
        assert_eq!(format_cost(1.5), "1.50");
    }

    #[test]
    fn test_get_context_color() {
        assert_eq!(get_context_color(90.0), colors::RED);
        assert_eq!(get_context_color(70.0), colors::YELLOW);
        assert_eq!(get_context_color(30.0), colors::GREEN);
    }

    #[test]
    fn test_parse_input() {
        let json = r#"{
            "hook_event_name": "Status",
            "model": {"display_name": "Opus"},
            "workspace": {"current_dir": "/test/project"},
            "context_window": {"used_percentage": 42.5},
            "cost": {"total_cost_usd": 0.0123}
        }"#;

        let input: StatusInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.model.display_name, Some("Opus".to_string()));
        assert_eq!(input.context_window.used_percentage, Some(42.5));
    }
}
