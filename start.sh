#!/bin/bash
# LogAI Quick Start Script

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color
BOLD='\033[1m'

echo ""
echo -e "${CYAN}${BOLD}╔═══════════════════════════════════════════════════════════╗${NC}"
echo -e "${CYAN}${BOLD}║             LogAI - AI-Powered Log Analysis               ║${NC}"
echo -e "${CYAN}${BOLD}╚═══════════════════════════════════════════════════════════╝${NC}"
echo ""

# Check prerequisites
check_prereqs() {
    echo -e "${YELLOW}Checking prerequisites...${NC}"
    
    # Check Docker
    if ! command -v docker &> /dev/null; then
        echo -e "${RED}✗ Docker not found. Please install Docker first.${NC}"
        exit 1
    fi
    echo -e "${GREEN}✓${NC} Docker found"
    
    # Check Docker Compose
    if ! docker compose version &> /dev/null; then
        echo -e "${RED}✗ Docker Compose not found. Please install Docker Compose.${NC}"
        exit 1
    fi
    echo -e "${GREEN}✓${NC} Docker Compose found"
    
    # Check Rust
    if ! command -v cargo &> /dev/null; then
        echo -e "${RED}✗ Rust not found. Please install Rust first.${NC}"
        echo "  Visit: https://rustup.rs"
        exit 1
    fi
    echo -e "${GREEN}✓${NC} Rust found"
    
    # Check .env for GROQ_API_KEY
    if [ ! -f .env ]; then
        echo -e "${YELLOW}! No .env file found. Creating template...${NC}"
        echo "GROQ_API_KEY=your-groq-api-key-here" > .env
        echo -e "${YELLOW}  Please edit .env and add your GROQ_API_KEY${NC}"
        echo -e "${YELLOW}  Get one free at: https://console.groq.com${NC}"
        exit 1
    fi
    
    if ! grep -q "GROQ_API_KEY=gsk_" .env 2>/dev/null; then
        echo -e "${YELLOW}! GROQ_API_KEY not configured properly in .env${NC}"
        echo -e "${YELLOW}  Please add your Groq API key to .env${NC}"
        echo -e "${YELLOW}  Get one free at: https://console.groq.com${NC}"
        exit 1
    fi
    echo -e "${GREEN}✓${NC} GROQ_API_KEY configured"
    
    echo ""
}

# Start infrastructure
start_infra() {
    echo -e "${YELLOW}Starting infrastructure services...${NC}"
    docker compose up -d
    
    echo -e "${YELLOW}Waiting for services to be ready...${NC}"
    sleep 5
    
    # Check NATS
    if curl -s http://localhost:8222/healthz > /dev/null 2>&1; then
        echo -e "${GREEN}✓${NC} NATS ready"
    else
        echo -e "${YELLOW}! NATS starting...${NC}"
    fi
    
    # Check ClickHouse
    if curl -s http://localhost:8123/ping > /dev/null 2>&1; then
        echo -e "${GREEN}✓${NC} ClickHouse ready"
    else
        echo -e "${YELLOW}! ClickHouse starting...${NC}"
    fi
    
    # Check Qdrant
    if curl -s http://localhost:6333/collections > /dev/null 2>&1; then
        echo -e "${GREEN}✓${NC} Qdrant ready"
    else
        echo -e "${YELLOW}! Qdrant starting...${NC}"
    fi
    
    echo ""
}

# Build project
build_project() {
    echo -e "${YELLOW}Building LogAI (this may take a few minutes on first run)...${NC}"
    cargo build --release 2>&1 | while read line; do
        if [[ "$line" == *"Compiling"* ]]; then
            echo -e "  ${CYAN}$line${NC}"
        fi
    done
    echo -e "${GREEN}✓${NC} Build complete"
    echo ""
}

# Start services
start_services() {
    echo -e "${YELLOW}Starting LogAI services...${NC}"
    echo ""
    
    # Start API in background
    echo -e "${CYAN}Starting API server on port 3000...${NC}"
    RUST_LOG=info ./target/release/logai-api &
    API_PID=$!
    echo $API_PID > /tmp/logai-api.pid
    sleep 2
    
    # Start Ingest worker in background
    echo -e "${CYAN}Starting ingestion worker...${NC}"
    RUST_LOG=info ./target/release/logai-worker &
    INGEST_PID=$!
    echo $INGEST_PID > /tmp/logai-ingest.pid
    sleep 2
    
    echo ""
}

# Show status
show_status() {
    echo -e "${GREEN}${BOLD}╔═══════════════════════════════════════════════════════════╗${NC}"
    echo -e "${GREEN}${BOLD}║                    LogAI is Running!                       ║${NC}"
    echo -e "${GREEN}${BOLD}╚═══════════════════════════════════════════════════════════╝${NC}"
    echo ""
    echo -e "${CYAN}API Server:${NC}    http://localhost:3000"
    echo -e "${CYAN}Health Check:${NC}  http://localhost:3000/health"
    echo ""
    echo -e "${BOLD}Quick Commands:${NC}"
    echo ""
    echo -e "  ${GREEN}Check status:${NC}"
    echo "    ./target/release/logai status"
    echo ""
    echo -e "  ${GREEN}Search logs:${NC}"
    echo "    ./target/release/logai search \"error timeout\""
    echo ""
    echo -e "  ${GREEN}Ask AI:${NC}"
    echo "    ./target/release/logai ask \"What are the common errors?\""
    echo ""
    echo -e "  ${GREEN}Ingest logs:${NC}"
    echo "    ./target/release/logai ingest /path/to/logs.log --format nginx --service my-app"
    echo ""
    echo -e "${YELLOW}To stop LogAI:${NC}  ./stop.sh"
    echo ""
}

# Main
main() {
    check_prereqs
    start_infra
    build_project
    start_services
    show_status
}

# Run
main
