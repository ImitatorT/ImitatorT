#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

compose_cmd=""
if docker compose version >/dev/null 2>&1; then
  compose_cmd="docker compose"
elif docker-compose version >/dev/null 2>&1; then
  compose_cmd="docker-compose"
else
  echo "未找到 docker compose 或 docker-compose，请先安装 Docker Desktop/Engine。" >&2
  exit 1
fi

if ! docker info >/dev/null 2>&1; then
  echo "Docker 未启动或当前用户无权限访问 Docker，请先启动 Docker。" >&2
  exit 1
fi

# 默认 tag（可以通过第 1 个参数覆盖）
AGENT_TAG="${1:-dev}"
# 预拉取镜像，避免在重启窗口内耗时
$compose_cmd pull conduwuit ghcr.io/zhengui666/imitatort-stateless-agent:"${AGENT_TAG}" >/dev/null 2>&1 || true

# 以最短停机窗口重建容器（先后端再框架）
$compose_cmd up -d --no-deps --no-build --force-recreate conduwuit
$compose_cmd up -d --no-deps --no-build --force-recreate swarms-agent

echo "部署完成。"
