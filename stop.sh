#!/bin/bash
# LogAI Stop Script

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo ""
echo -e "${YELLOW}Stopping LogAI services...${NC}"
echo ""

# Stop API
if [ -f /tmp/logai-api.pid ]; then
    PID=$(cat /tmp/logai-api.pid)
    if kill -0 $PID 2>/dev/null; then
        kill $PID
        echo -e "${GREEN}✓${NC} API server stopped"
    fi
    rm -f /tmp/logai-api.pid
fi

# Stop Ingest worker
if [ -f /tmp/logai-ingest.pid ]; then
    PID=$(cat /tmp/logai-ingest.pid)
    if kill -0 $PID 2>/dev/null; then
        kill $PID
        echo -e "${GREEN}✓${NC} Ingestion worker stopped"
    fi
    rm -f /tmp/logai-ingest.pid
fi

# Also kill by name if PIDs not found
pkill -f logai-api 2>/dev/null || true
pkill -f logai-worker 2>/dev/null || true

echo ""
echo -e "${YELLOW}Stop infrastructure? (docker compose down) [y/N]${NC}"
read -r response
if [[ "$response" =~ ^([yY][eE][sS]|[yY])$ ]]; then
    docker compose down
    echo -e "${GREEN}✓${NC} Infrastructure stopped"
fi

echo ""
echo -e "${GREEN}LogAI stopped.${NC}"
echo ""
