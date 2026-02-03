# Claude Code Statusline (Rust)

一个用 Rust 实现的 Claude Code statusline 插件，显示模型、目录、上下文使用率、成本等信息。

## 效果预览

```
[Opus] │ my-project │ ctx:42% │ in:15.2k │ $0.012 │ +156/-23
```

## 安装

```bash
# 克隆并构建
cd /path/to/claudecode-statusline
./install.sh
```

或手动安装：

```bash
cargo build --release
cp target/release/claudecode-statusline ~/.claude/
chmod +x ~/.claude/claudecode-statusline
```

然后在 `~/.claude/settings.json` 中添加：

```json
{
  "statusLine": {
    "type": "command",
    "command": "~/.claude/claudecode-statusline",
    "padding": 0
  }
}
```

## 显示内容

| 项目 | 说明 | 颜色 |
|------|------|------|
| `[Model]` | 当前模型名称 | 紫色 |
| 目录名 | 当前工作目录 | 青色 |
| `ctx:N%` | 上下文窗口使用率 | 绿/黄/红 |
| `in:Nk` | 输入 token 数 | 灰色 |
| `$N.NN` | 会话成本 (USD) | 黄色 |
| `+N/-N` | 代码行变更 | 绿/红 |

上下文使用率颜色：
- 绿色: < 60%
- 黄色: 60-80%
- 红色: > 80%

## 自定义

修改 `src/main.rs` 中的 `build_statusline` 函数来自定义显示内容。

## License

MIT
