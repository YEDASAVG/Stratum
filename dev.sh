#!/bin/bash
# Dev mode: Auto-restart on file changes (like nodemon)

set -e

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
RED='\033[0;31m'
NC='\033[0m'

cd "$(dirname "$0")"
export $(grep -v '^#' .env | xargs)

kill_services() {
  pkill -f "logai-api" 2>/dev/null || true
  pkill -f "logai-worker" 2>/dev/null || true
}

start_services() {
  RUST_LOG=info ./target/release/logai-api &
  sleep 1
  RUST_LOG=info ./target/release/logai-worker &
}

case "${1:-watch}" in
  watch|"")
    echo -e "${CYAN}══════════════════════════════════════${NC}"
    echo -e "${CYAN}   LogAI Dev Mode (Auto-Restart)      ${NC}"
    echo -e "${CYAN}══════════════════════════════════════${NC}"
    echo -e "${YELLOW}Watching for changes... (Ctrl+C to stop)${NC}"
    echo ""
    
    cargo build --release 2>&1 | grep -E "Compiling|Finished" || true
    kill_services
    start_services
    echo -e "${GREEN}✓ Services started${NC}"
    echo ""
    
    cargo watch -w crates -s "
      echo '🔄 Rebuilding...'
      cargo build --release 2>&1 | grep -E 'Compiling|Finished|error' || true
      pkill -f logai-api 2>/dev/null || true
      pkill -f logai-worker 2>/dev/null || true
      sleep 1
      RUST_LOG=info ./target/release/logai-api &
      RUST_LOG=info ./target/release/logai-worker &
      echo '✅ Restarted!'
    "
    ;;
    
  once)
    cargo build --release
    kill_services
    sleep 1
    start_services
    echo -e "${GREEN}✓ Running at http://localhost:3000${NC}"
    ;;
    
  stop)
    kill_services
    echo -e "${GREEN}✓ Stopped${NC}"
    ;;
    
  status)
    pgrep -f "logai-api" > /dev/null && echo -e "${GREEN}✓ API running${NC}" || echo -e "${RED}✗ API stopped${NC}"
    pgrep -f "logai-worker" > /dev/null && echo -e "${GREEN}✓ Worker running${NC}" || echo -e "${RED}✗ Worker stopped${NC}"
    ;;
    
  *)
    echo "Usage: ./dev.sh [watch|once|stop|status]"
    ;;
esac
