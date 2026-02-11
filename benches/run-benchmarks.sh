#!/bin/bash
# LogAI Benchmark Suite
# Run all benchmarks and generate reports

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR/.."

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${BLUE}╔═══════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║           LogAI Benchmark Suite                           ║${NC}"
echo -e "${BLUE}╚═══════════════════════════════════════════════════════════╝${NC}"
echo

# Parse arguments
RUN_CRITERION=false
RUN_STRESS=false
RUN_K6=false
ALL=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --criterion)
            RUN_CRITERION=true
            shift
            ;;
        --stress)
            RUN_STRESS=true
            shift
            ;;
        --k6)
            RUN_K6=true
            shift
            ;;
        --all)
            ALL=true
            shift
            ;;
        --help)
            echo "Usage: $0 [OPTIONS]"
            echo
            echo "Options:"
            echo "  --criterion   Run Criterion microbenchmarks (parsing, rag)"
            echo "  --stress      Run stress test (ingestion throughput)"
            echo "  --k6          Run k6 load test (API endpoints)"
            echo "  --all         Run all benchmarks"
            echo "  --help        Show this help"
            echo
            echo "Examples:"
            echo "  $0 --criterion          # Run parsing and RAG benchmarks"
            echo "  $0 --stress             # Run 100K log stress test"
            echo "  $0 --all                # Run everything"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Default to criterion if no args
if [[ "$RUN_CRITERION" == false && "$RUN_STRESS" == false && "$RUN_K6" == false && "$ALL" == false ]]; then
    RUN_CRITERION=true
fi

if [[ "$ALL" == true ]]; then
    RUN_CRITERION=true
    RUN_STRESS=true
    RUN_K6=true
fi

# Criterion benchmarks
if [[ "$RUN_CRITERION" == true ]]; then
    echo -e "${GREEN}▶ Running Criterion Benchmarks...${NC}"
    echo
    
    echo -e "${YELLOW}  → Parsing benchmarks (logai-core)${NC}"
    cargo bench -p logai-core --bench parsing 2>&1 | tail -50
    
    echo
    echo -e "${YELLOW}  → RAG benchmarks (logai-rag)${NC}"
    cargo bench -p logai-rag --bench rag 2>&1 | tail -50
    
    echo
    echo -e "${GREEN}✓ Criterion reports saved to: target/criterion/report/index.html${NC}"
fi

# Stress test
if [[ "$RUN_STRESS" == true ]]; then
    echo
    echo -e "${GREEN}▶ Running Stress Test...${NC}"
    
    # Check if API is running
    if ! curl -s http://localhost:3000/health > /dev/null 2>&1; then
        echo -e "${YELLOW}  ⚠ API not running. Start with: cargo run --release --bin logai-api${NC}"
        echo "  Skipping stress test."
    else
        echo -e "${YELLOW}  → Running 100K log ingestion test${NC}"
        cargo run --release --bin logai-stress -- \
            --rate 50000 \
            --total 100000 \
            --batch 500 \
            --workers 50 \
            --endpoint http://localhost:3000
    fi
fi

# k6 load test
if [[ "$RUN_K6" == true ]]; then
    echo
    echo -e "${GREEN}▶ Running k6 Load Test...${NC}"
    
    # Check if k6 is installed
    if ! command -v k6 &> /dev/null; then
        echo -e "${YELLOW}  ⚠ k6 not installed. Install with: brew install k6${NC}"
        echo "  Skipping k6 load test."
    else
        # Check if API is running
        if ! curl -s http://localhost:3000/health > /dev/null 2>&1; then
            echo -e "${YELLOW}  ⚠ API not running. Start with: cargo run --release --bin logai-api${NC}"
            echo "  Skipping k6 load test."
        else
            echo -e "${YELLOW}  → Running 60s load test with mixed workload${NC}"
            k6 run --summary-trend-stats="avg,min,med,max,p(90),p(95),p(99)" \
                benches/loadtest.js
        fi
    fi
fi

echo
echo -e "${BLUE}╔═══════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║           Benchmark Complete!                             ║${NC}"
echo -e "${BLUE}╚═══════════════════════════════════════════════════════════╝${NC}"
