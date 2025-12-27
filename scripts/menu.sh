#!/usr/bin/env bash
# EXLO Interactive Command Menu
# Requires: gum (https://github.com/charmbracelet/gum)

set -euo pipefail

# Colors
CYAN='\033[0;36m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Check if gum is installed
check_gum() {
    if ! command -v gum &> /dev/null; then
        echo -e "${RED}Error: gum is not installed${NC}"
        echo ""
        echo "Install gum:"
        echo "  macOS:  brew install gum"
        echo "  Linux:  See https://github.com/charmbracelet/gum#installation"
        exit 1
    fi
}

# Header
show_header() {
    gum style \
        --foreground 212 --border-foreground 212 --border double \
        --align center --width 50 --margin "1 2" --padding "1 2" \
        'EXLO Command Menu' 'Self-Hosted SSH Reverse Tunnel'
}

# Main menu categories
main_menu() {
    CATEGORY=$(gum choose \
        "Development" \
        "Build" \
        "Deploy" \
        "Database" \
        "Status" \
        "Cleanup" \
        "Exit")

    case "$CATEGORY" in
        "Development") dev_menu ;;
        "Build") build_menu ;;
        "Deploy") deploy_menu ;;
        "Database") db_menu ;;
        "Status") status_menu ;;
        "Cleanup") cleanup_menu ;;
        "Exit") exit 0 ;;
    esac
}

# Development submenu
dev_menu() {
    CMD=$(gum choose \
        "dev-web         Start Web dashboard" \
        "dev-tunnl       Start Tunnl (Rust)" \
        "dev-test-server Start local test HTTP server" \
        "dev-ssh-client  Test SSH reverse tunnel" \
        "dev-landing     Start Landing page" \
        "dev-docs        Start Documentation" \
        "dev-all         Start all services" \
        "init-dev-db     Initialize dev database" \
        "<- Back")

    run_command "$CMD"
}

# Build submenu
build_menu() {
    CMD=$(gum choose \
        "build         Build all applications" \
        "build-web     Build Web only" \
        "build-tunnl   Build Tunnl only" \
        "build-images  Build Docker images" \
        "<- Back")

    run_command "$CMD"
}

# Deploy submenu
deploy_menu() {
    CMD=$(gum choose \
        "deploy-simple Deploy (HTTP, no domain)" \
        "deploy-local  Deploy (Traefik HTTP)" \
        "deploy-prod   Deploy (HTTPS + SSL)" \
        "up            Start services" \
        "up-build      Start with rebuild" \
        "down          Stop all services" \
        "<- Back")

    run_command "$CMD"
}

# Database submenu
db_menu() {
    CMD=$(gum choose \
        "init-db       Initialize database" \
        "db-backup     Backup database" \
        "db-restore    Restore database" \
        "<- Back")

    run_command "$CMD"
}

# Status submenu
status_menu() {
    CMD=$(gum choose \
        "health        Check service health" \
        "status        Detailed status" \
        "ps            Show containers" \
        "logs          View all logs" \
        "logs-web      View Web logs" \
        "logs-tunnl    View Tunnl logs" \
        "<- Back")

    run_command "$CMD"
}

# Cleanup submenu
cleanup_menu() {
    CMD=$(gum choose \
        "clean         Full cleanup (dangerous)" \
        "clean-images  Remove images only" \
        "clean-volumes Remove volumes only" \
        "<- Back")

    run_command "$CMD"
}

# Run the selected command
run_command() {
    local selection="$1"

    if [[ "$selection" == "<- Back" ]]; then
        return
    fi

    # Extract command name (first word)
    local cmd
    cmd=$(awk '{print $1}' <<<"$selection")

    echo ""
    echo -e "${CYAN}Running: make $cmd${NC}"
    echo ""

    # Run the command
    if ! make "$cmd"; then
        echo -e "${RED}make $cmd failed${NC}"
        exit 1
    fi

    echo ""
    if gum confirm "Continue?"; then
        return
    else
        exit 0
    fi
}

# Main
check_gum
show_header

while true; do
    main_menu
done
