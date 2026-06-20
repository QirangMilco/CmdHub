#!/usr/bin/env bash
set -euo pipefail

# ─────────────────────────────────────────────────────────────
# CmdHub 发布脚本 —— 创建并推送 git tag
#
# 用法:
#   ./scripts/release.sh v0.1.3
#   ./scripts/release.sh 0.1.3          # 自动补 v 前缀
#   ./scripts/release.sh v0.1.4 --force  # 跳过确认
# ─────────────────────────────────────────────────────────────

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

error()  { echo -e "${RED}错误:${NC} $*" >&2; }
info()   { echo -e "${GREEN}$*${NC}"; }
warn()   { echo -e "${YELLOW}警告:${NC} $*"; }

# ── 参数解析 ──

TAG=""
FORCE=false
for arg in "$@"; do
  case "$arg" in
    --force|-f) FORCE=true ;;
    *)
      if [[ -z "$TAG" ]]; then
        TAG="$arg"
      fi
      ;;
  esac
done

if [[ -z "$TAG" ]]; then
  echo "用法: $0 <tag名或版本号> [--force]"
  echo "示例: $0 v0.1.3"
  echo "     $0 0.1.3          （自动补 v 前缀）"
  exit 1
fi

# 如果没有 v 前缀，自动补上
if [[ ! "$TAG" =~ ^v ]]; then
  TAG="v$TAG"
fi

# ── 检查 tag 状态 ──

LOCAL_EXISTS=false
REMOTE_EXISTS=false

if git tag -l | grep -q "^$TAG$"; then
  LOCAL_EXISTS=true
fi

if git ls-remote --tags origin 2>/dev/null | grep -q "refs/tags/$TAG$"; then
  REMOTE_EXISTS=true
fi

# ── 如果 tag 已存在 → 显示信息并确认 ──

if $LOCAL_EXISTS || $REMOTE_EXISTS; then
  echo ""
  info "  tag $TAG 已存在"

  if $LOCAL_EXISTS; then
    echo ""
    info "  ── 本地 tag 信息 ──"
    git show "$TAG" --stat --no-notes | head -30
    echo ""
  fi

  if $REMOTE_EXISTS; then
    echo ""
    info "  ── 远端 tag 信息 ──"
    git ls-remote --tags origin 2>/dev/null | grep "refs/tags/$TAG$"
    echo ""
  fi

  if ! $FORCE; then
    echo -n "是否删除并重新推送 tag ${TAG}？[y/N] "
    read -r CONFIRM
    if [[ ! "$CONFIRM" =~ ^[Yy]$ ]]; then
      info "已取消"
      exit 0
    fi
  fi

  # 删除本地 tag
  if $LOCAL_EXISTS; then
    git tag -d "$TAG"
    info "本地 tag $TAG 已删除"
  fi

  # 删除远端 tag
  if $REMOTE_EXISTS; then
    git push origin --delete "$TAG"
    info "远端 tag $TAG 已删除"
  fi

  echo ""
fi

# ── 创建并推送 tag ──

echo ""
info "创建 tag $TAG ← $(git rev-parse --short HEAD)"
git tag "$TAG"

echo ""
info "推送 tag $TAG → origin"
git push origin "$TAG"

echo ""
info "✓ tag $TAG 已推送"
echo ""
echo "打开 Release 页面："
echo "  https://github.com/liliMozi/CmdHub/releases/tag/$TAG"
