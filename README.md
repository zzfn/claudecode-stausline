# Claude Code Statusline (Rust)

一个用 Rust 实现的 Claude Code statusline 插件，显示模型、目录、上下文使用率、成本等信息。

## 效果预览

```
[Opus] │ my-project │ main │ ctx:42% │ in:15.2k │ cache:85% │ [ZAI] Token(5h):1% │ [ZAI] MCP(1月):0%
```

## 安装

### 方式一：一键安装（推荐，仅限 Linux/macOS）

```bash
curl -fsSL https://raw.githubusercontent.com/zzfn/cc-statusline/main/setup.sh | bash
```

### 方式二：从源码构建

```bash
git clone https://github.com/zzfn/cc-statusline.git
cd cc-statusline
./install.sh  # Linux/macOS
# 或在 Windows 上使用: cargo build --release
```

### 方式三：手动安装

#### Linux/macOS

1. 从 [Releases](https://github.com/zzfn/cc-statusline/releases) 下载对应平台的二进制文件
2. 解压并复制到 `~/.claude/`
3. 在 `~/.claude/settings.json` 中添加：

```json
{
  "statusLine": {
    "type": "command",
    "command": "~/.claude/cc-statusline",
    "padding": 0
  }
}
```

#### Windows

1. 从 [Releases](https://github.com/zzfn/cc-statusline/releases) 下载 `cc-statusline-x86_64-pc-windows-msvc.zip`
2. 解压到 `%USERPROFILE%\.claude\` 目录
3. 在 `%USERPROFILE%\.claude\settings.json` 中添加：

```json
{
  "statusLine": {
    "type": "command",
    "command": "%USERPROFILE%\\.claude\\cc-statusline.exe",
    "padding": 0
  }
}
```

## ZAI 功能

如果你使用 ZAI API，只需在 Claude Code 配置文件 `~/.claude/settings.json` 中设置 `baseURL` 和 `authToken`，或设置环境变量：

```bash
export ANTHROPIC_BASE_URL="https://api.z.ai/api/anthropic"
# 或
export ANTHROPIC_BASE_URL="https://open.bigmodel.cn/api/anthropic"

export ANTHROPIC_AUTH_TOKEN="your-token-here"
```

程序会自动检测并显示 ZAI 的 Token 使用率和 MCP 使用率。

## 显示内容

| 项目 | 说明 | 颜色 |
|------|------|------|
| `[Model]` | 当前模型名称 | 紫色 |
| 目录名 | 当前工作目录 | 青色 |
| Git 分支 | 当前 git 分支 | 蓝色 |
| `ctx:N%` | 上下文窗口使用率 | 绿/黄/红 |
| `in:Nk` | 输入 token 数 | 灰色 |
| `cache:N%` | 缓存命中率 | 绿/黄/红 |
| `[ZAI] Token(5h):N%` | ZAI Token 使用率（5小时窗口） | 绿/黄/红 |
| `[ZAI] MCP(1月):N%` | ZAI MCP 调用使用率（1个月窗口） | 绿/黄/红 |

上下文使用率颜色：
- 绿色: < 60%
- 黄色: 60-80%
- 红色: > 80%

缓存命中率颜色：
- 绿色: ≥ 80%
- 黄色: 50-80%
- 红色: < 50%

ZAI 使用率颜色：
- 绿色: < 60%
- 黄色: 60-80%
- 红色: > 80%

## 自定义

修改 `src/main.rs` 中的 `build_statusline` 函数来自定义显示内容。

## License

MIT
