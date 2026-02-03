#!/bin/bash
# Claude Code Statusline 安装脚本

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BINARY_NAME="claudecode-statusline"
INSTALL_DIR="$HOME/.claude"

echo "构建 release 版本..."
cargo build --release

echo "创建 ~/.claude 目录..."
mkdir -p "$INSTALL_DIR"

echo "复制二进制文件..."
cp "$SCRIPT_DIR/target/release/$BINARY_NAME" "$INSTALL_DIR/"
chmod +x "$INSTALL_DIR/$BINARY_NAME"

echo "配置 Claude Code..."
SETTINGS_FILE="$INSTALL_DIR/settings.json"

if [ -f "$SETTINGS_FILE" ]; then
    # 使用 jq 更新现有配置（如果有 jq）
    if command -v jq &> /dev/null; then
        TMP_FILE=$(mktemp)
        jq '.statusLine = {"type": "command", "command": "~/.claude/claudecode-statusline", "padding": 0}' "$SETTINGS_FILE" > "$TMP_FILE"
        mv "$TMP_FILE" "$SETTINGS_FILE"
        echo "已更新 settings.json"
    else
        echo "警告: 未找到 jq，请手动更新 $SETTINGS_FILE"
        echo "添加以下配置:"
        echo '  "statusLine": {"type": "command", "command": "~/.claude/claudecode-statusline", "padding": 0}'
    fi
else
    # 创建新配置文件
    cat > "$SETTINGS_FILE" << 'EOF'
{
  "statusLine": {
    "type": "command",
    "command": "~/.claude/claudecode-statusline",
    "padding": 0
  }
}
EOF
    echo "已创建 settings.json"
fi

echo ""
echo "✓ 安装完成！"
echo "  二进制文件: $INSTALL_DIR/$BINARY_NAME"
echo "  配置文件: $SETTINGS_FILE"
echo ""
echo "重启 Claude Code 即可生效。"
