use serde::Deserialize;
use std::io::{self, Read};
use std::process::Command;

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
    path.rsplit('/').next().unwrap_or(path)
}

/// 获取当前 git 分支名
fn get_git_branch(cwd: Option<&str>) -> Option<String> {
    let output = Command::new("git")
        .args(&["branch", "--show-current"])
        .current_dir(cwd.unwrap_or("."))
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
    if let Some(percentage) = input.context_window.used_percentage {
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
    }

    // 成本
    if let Some(cost) = input.cost.total_cost_usd {
        if cost > 0.0 {
            parts.push(format!(
                "{}${}{}",
                colors::YELLOW,
                format_cost(cost),
                colors::RESET
            ));
        }
    }

    // 代码变更统计
    let lines_added = input.cost.total_lines_added.unwrap_or(0);
    let lines_removed = input.cost.total_lines_removed.unwrap_or(0);
    if lines_added > 0 || lines_removed > 0 {
        parts.push(format!(
            "{}+{}{}/-{}{}",
            colors::GREEN,
            lines_added,
            colors::RED,
            lines_removed,
            colors::RESET
        ));
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
