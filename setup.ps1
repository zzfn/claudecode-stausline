# Claude Code Statusline Windows 安装脚本
# 从 GitHub Release 下载并配置

param(
    [switch]$Help
)

$ErrorActionPreference = "Stop"

$REPO = "zzfn/cc-statusline"
$BINARY_NAME = "cc-statusline.exe"
$INSTALL_DIR = "$env:USERPROFILE\.claude"

# 显示帮助信息
if ($Help) {
    Write-Host "用法: .\setup.ps1 [选项]"
    Write-Host ""
    Write-Host "选项:"
    Write-Host "  -Help             显示此帮助信息"
    exit 0
}

# 获取最新 release 版本
function Get-LatestVersion {
    try {
        $response = Invoke-RestMethod -Uri "https://api.github.com/repos/$REPO/releases/latest"
        return $response.tag_name
    } catch {
        return $null
    }
}

Write-Host "=== Claude Code Statusline 安装脚本 ===" -ForegroundColor Cyan
Write-Host ""

# 检测平台
$PLATFORM = "x86_64-pc-windows-msvc"
Write-Host "检测到平台: $PLATFORM" -ForegroundColor Green

# 获取最新版本
Write-Host "获取最新版本..."
$VERSION = Get-LatestVersion

if ([string]::IsNullOrEmpty($VERSION)) {
    Write-Host "警告: 无法获取最新版本，使用 latest" -ForegroundColor Yellow
    $DOWNLOAD_URL = "https://github.com/$REPO/releases/latest/download/cc-statusline-$PLATFORM.zip"
} else {
    Write-Host "最新版本: $VERSION" -ForegroundColor Green
    $DOWNLOAD_URL = "https://github.com/$REPO/releases/download/$VERSION/cc-statusline-$PLATFORM.zip"
}

# 创建安装目录
Write-Host "创建安装目录: $INSTALL_DIR"
if (-not (Test-Path $INSTALL_DIR)) {
    New-Item -ItemType Directory -Path $INSTALL_DIR -Force | Out-Null
}

# 下载并解压
Write-Host "下载中..."
$TMP_DIR = [System.IO.Path]::GetTempPath() + [System.Guid]::NewGuid().ToString()
New-Item -ItemType Directory -Path $TMP_DIR -Force | Out-Null

try {
    $ZIP_FILE = "$TMP_DIR\release.zip"
    Invoke-WebRequest -Uri $DOWNLOAD_URL -OutFile $ZIP_FILE -UseBasicParsing

    # 解压
    Expand-Archive -Path $ZIP_FILE -DestinationPath $TMP_DIR -Force

    # 移动二进制文件
    Move-Item -Path "$TMP_DIR\$BINARY_NAME" -Destination "$INSTALL_DIR\$BINARY_NAME" -Force

    Write-Host "已安装到: $INSTALL_DIR\$BINARY_NAME" -ForegroundColor Green
} catch {
    Write-Host "错误: 下载或安装失败" -ForegroundColor Red
    Write-Host $_.Exception.Message -ForegroundColor Red
    exit 1
} finally {
    # 清理临时文件
    if (Test-Path $TMP_DIR) {
        Remove-Item -Path $TMP_DIR -Recurse -Force
    }
}

# 配置 settings.json
$SETTINGS_FILE = "$INSTALL_DIR\settings.json"
$BINARY_PATH = "$INSTALL_DIR\$BINARY_NAME" -replace '\\', '\\'

if (Test-Path $SETTINGS_FILE) {
    # 读取现有配置
    try {
        $settings = Get-Content $SETTINGS_FILE -Raw | ConvertFrom-Json

        # 更新 statusLine 配置
        $settings | Add-Member -MemberType NoteProperty -Name "statusLine" -Value @{
            type = "command"
            command = "$INSTALL_DIR\$BINARY_NAME"
            padding = 0
        } -Force

        # 写回文件
        $settings | ConvertTo-Json -Depth 10 | Set-Content $SETTINGS_FILE -Encoding UTF8
        Write-Host "已更新配置: $SETTINGS_FILE" -ForegroundColor Green
    } catch {
        Write-Host "警告: 无法自动更新配置，请手动添加以下内容到 $SETTINGS_FILE :" -ForegroundColor Yellow
        Write-Host '  "statusLine": {"type": "command", "command": "' + "$INSTALL_DIR\$BINARY_NAME" + '", "padding": 0}'
    }
} else {
    # 创建新配置文件
    $config = @{
        statusLine = @{
            type = "command"
            command = "$INSTALL_DIR\$BINARY_NAME"
            padding = 0
        }
    }

    $config | ConvertTo-Json -Depth 10 | Set-Content $SETTINGS_FILE -Encoding UTF8
    Write-Host "已创建配置: $SETTINGS_FILE" -ForegroundColor Green
}

Write-Host ""
Write-Host "✓ 安装完成！" -ForegroundColor Green
Write-Host ""
Write-Host "重启 Claude Code 或配置会自动生效。"
Write-Host ""
Write-Host "提示: 如果使用 ZAI API，程序会自动检测并显示使用情况。"
Write-Host "只需在 $SETTINGS_FILE 中配置 baseURL 和 authToken 即可。"
