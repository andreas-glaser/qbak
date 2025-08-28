#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

run_docker_compose() {
    cd "$SCRIPT_DIR"
    if command -v docker compose &> /dev/null; then
        docker compose "$@"
    elif command -v docker-compose &> /dev/null; then
        docker-compose "$@"
    else
        echo -e "${RED}Docker Compose not found${NC}"
        exit 1
    fi
}

run_cargo() {
    run_docker_compose run --rm qbak cargo "$@"
}

case "$1" in
    dev)
        echo -e "${GREEN}Starting development shell...${NC}"
        run_docker_compose run --rm qbak
        ;;
    
    build)
        echo -e "${GREEN}Building release binary...${NC}"
        run_cargo build --release --target x86_64-unknown-linux-musl
        ;;
    
    test)
        echo -e "${GREEN}Running tests...${NC}"
        run_cargo test -- --test-threads=1
        ;;
    
    fmt)
        echo -e "${GREEN}Formatting code...${NC}"
        run_cargo fmt --all
        ;;
    
    clippy)
        echo -e "${GREEN}Running Clippy...${NC}"
        run_cargo clippy --all-targets --all-features -- -D warnings
        ;;
    
    pre-commit)
        echo -e "${GREEN}Running pre-commit checks...${NC}"
        run_docker_compose run --rm qbak bash -c "
            cargo fmt --all -- --check &&
            cargo clippy --all-targets --all-features -- -D warnings &&
            cargo test -- --test-threads=1
        "
        ;;
    
    clean)
        echo -e "${YELLOW}Stopping dev container and removing volumes...${NC}"
        run_docker_compose down -v --remove-orphans
        ;;

    
    *)
        echo "qbak Docker Helper"
        echo ""
        echo "Usage: $0 <command>"
        echo ""
        echo "Commands:"
        echo "  dev        - Start development shell"
        echo "  build      - Build release binary" 
        echo "  test       - Run tests"
        echo "  fmt        - Format code"
        echo "  clippy     - Run linter"
        echo "  pre-commit - Run all checks"
        echo "  clean      - Stop and remove project volumes"
        ;;
esac
