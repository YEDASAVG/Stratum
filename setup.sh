#!/bin/bash
# LogAI One-Command Setup
# Usage: ./setup.sh

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m'

echo ""
echo -e "${CYAN}${BOLD}╔═══════════════════════════════════════════════════════════╗${NC}"
echo -e "${CYAN}${BOLD}║       LogAI - AI-Powered Log Intelligence                 ║${NC}"
echo -e "${CYAN}${BOLD}║           One-Command Docker Setup                         ║${NC}"
echo -e "${CYAN}${BOLD}╚═══════════════════════════════════════════════════════════╝${NC}"
echo ""

# Check Docker
if ! command -v docker &> /dev/null; then
    echo -e "${RED}✗ Docker not found. Please install Docker first.${NC}"
    echo "  Visit: https://docs.docker.com/get-docker/"
    exit 1
fi
echo -e "${GREEN}✓${NC} Docker found"

# Check Docker Compose
if ! docker compose version &> /dev/null; then
    echo -e "${RED}✗ Docker Compose not found.${NC}"
    exit 1
fi
echo -e "${GREEN}✓${NC} Docker Compose found"

# Check/Create .env file
if [ ! -f .env ]; then
    echo ""
    echo -e "${YELLOW}Creating .env file from template...${NC}"
    cp .env.example .env
    
    echo ""
    echo -e "${YELLOW}${BOLD}⚠️  GROQ API KEY REQUIRED${NC}"
    echo -e "   Get a free API key at: ${CYAN}https://console.groq.com${NC}"
    echo ""
    read -p "   Enter your Groq API key (gsk_...): " GROQ_KEY
    
    if [[ "$GROQ_KEY" == gsk_* ]]; then
        # Replace the placeholder in .env
        if [[ "$OSTYPE" == "darwin"* ]]; then
            sed -i '' "s/GROQ_API_KEY=.*/GROQ_API_KEY=$GROQ_KEY/" .env
        else
            sed -i "s/GROQ_API_KEY=.*/GROQ_API_KEY=$GROQ_KEY/" .env
        fi
        echo -e "${GREEN}✓${NC} API key saved to .env"
    else
        echo -e "${RED}✗ Invalid API key format. Please edit .env manually.${NC}"
        exit 1
    fi
fi

# Verify GROQ_API_KEY is set
if ! grep -q "GROQ_API_KEY=gsk_" .env 2>/dev/null; then
    echo -e "${RED}✗ GROQ_API_KEY not configured in .env${NC}"
    echo -e "   Please edit .env and add your Groq API key"
    exit 1
fi
echo -e "${GREEN}✓${NC} Groq API key configured"

echo ""
echo -e "${YELLOW}Building and starting services (this may take 5-10 minutes on first run)...${NC}"
echo ""

# Build and start all services
docker compose build --parallel

echo ""
echo -e "${YELLOW}Starting services...${NC}"
docker compose up -d

# Wait for services to be healthy
echo ""
echo -e "${YELLOW}Waiting for services to be ready...${NC}"

# Wait for API to be healthy (max 60 seconds)
ATTEMPTS=0
MAX_ATTEMPTS=60
while [ $ATTEMPTS -lt $MAX_ATTEMPTS ]; do
    if curl -s http://localhost:3000/health > /dev/null 2>&1; then
        break
    fi
    ATTEMPTS=$((ATTEMPTS + 1))
    sleep 1
    echo -ne "\r  Waiting for API... ($ATTEMPTS/$MAX_ATTEMPTS)"
done
echo ""

if [ $ATTEMPTS -eq $MAX_ATTEMPTS ]; then
    echo -e "${RED}✗ API failed to start. Check logs with: docker compose logs api${NC}"
    exit 1
fi

echo ""
echo -e "${GREEN}${BOLD}╔═══════════════════════════════════════════════════════════╗${NC}"
echo -e "${GREEN}${BOLD}║                 🚀 LogAI is Ready!                         ║${NC}"
echo -e "${GREEN}${BOLD}╚═══════════════════════════════════════════════════════════╝${NC}"
echo ""
echo -e "  ${CYAN}Dashboard:${NC}  http://localhost:3001"
echo -e "  ${CYAN}API:${NC}        http://localhost:3000"
echo -e "  ${CYAN}Health:${NC}     http://localhost:3000/health"
echo ""
echo -e "${YELLOW}Commands:${NC}"
echo -e "  docker compose logs -f        # View logs"
echo -e "  docker compose down            # Stop all services"
echo -e "  docker compose --profile demo up -d  # Start with log simulator"
echo ""

# Try to open browser
if [[ "$OSTYPE" == "darwin"* ]]; then
    open http://localhost:3001 2>/dev/null || true
elif command -v xdg-open &> /dev/null; then
    xdg-open http://localhost:3001 2>/dev/null || true
fi
