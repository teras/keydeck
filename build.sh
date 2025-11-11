#!/bin/bash
set -e

# Docker-based build script for KeyDeck binaries
# Builds keydeck (CLI) and keydeck-config (UI) as standalone binaries
# No packaging - just raw executables
#
# Usage:
#   ./build.sh app    - Build binaries
#   ./build.sh clean  - Clean all build artifacts and return to fresh state
#   ./build.sh help   - Show help message

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Get directories
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$SCRIPT_DIR"
DIST_DIR="$PROJECT_ROOT/dist"
DOCKER_DIR="$PROJECT_ROOT/docker"

# Show help function
show_help() {
    echo "======================================"
    echo "KeyDeck Build Script"
    echo "======================================"
    echo ""
    echo "Usage: ./build.sh <command>"
    echo ""
    echo "Commands:"
    echo "  app     - Build both daemon and UI binaries"
    echo "  daemon  - Build only keydeck daemon (CLI) binary"
    echo "  ui      - Build only keydeck-config UI binary"
    echo "  install - Stop service, copy binaries to ~/Works/System/bin, and restart service"
    echo "  clean   - Remove all build artifacts and return to fresh state"
    echo "  check   - Check for missing license headers in source files"
    echo "  help    - Show this help message"
    echo ""
    echo "Examples:"
    echo "  ./build.sh app"
    echo "  ./build.sh daemon"
    echo "  ./build.sh ui"
    echo "  ./build.sh install"
    echo "  ./build.sh clean"
    echo "  ./build.sh check"
    echo ""
}

# Handle commands
case "$1" in
    help|--help|-h|"")
        show_help
        exit 0
        ;;
    clean|check|install)
        # Handle clean, check, and install commands below
        ;;
    app|daemon|ui)
        # Handle build commands below
        ;;
    *)
        echo -e "${RED}Error: Unknown command '$1'${NC}"
        echo ""
        show_help
        exit 1
        ;;
esac

# Handle check command
if [ "$1" = "check" ]; then
    echo "======================================"
    echo "KeyDeck License Header Check"
    echo "======================================"
    echo ""

    MISSING_FILES=0

    echo -e "${BLUE}Checking keydeck source files...${NC}"
    for file in "$PROJECT_ROOT/src"/*.rs "$PROJECT_ROOT/build.rs"; do
        if [ -f "$file" ]; then
            if ! grep -q "SPDX-License-Identifier: GPL-3.0-or-later" "$file"; then
                echo -e "${RED}✗ Missing GPL header: $file${NC}"
                MISSING_FILES=$((MISSING_FILES + 1))
            fi
        fi
    done

    echo ""
    echo -e "${BLUE}Checking keydeck-config Rust source files...${NC}"
    for file in "$PROJECT_ROOT/keydeck-config/src-tauri/src"/*.rs "$PROJECT_ROOT/keydeck-config/src-tauri/build.rs"; do
        if [ -f "$file" ]; then
            if ! grep -q "SPDX-License-Identifier: AGPL-3.0-or-later" "$file"; then
                echo -e "${RED}✗ Missing AGPL header: $file${NC}"
                MISSING_FILES=$((MISSING_FILES + 1))
            fi
        fi
    done

    echo ""
    echo -e "${BLUE}Checking keydeck-config frontend files (TypeScript/JavaScript)...${NC}"
    while IFS= read -r -d '' file; do
        if ! grep -q "SPDX-License-Identifier: AGPL-3.0-or-later" "$file"; then
            echo -e "${RED}✗ Missing AGPL header: $file${NC}"
            MISSING_FILES=$((MISSING_FILES + 1))
        fi
    done < <(find "$PROJECT_ROOT/keydeck-config/src" -type f \( -name "*.js" -o -name "*.ts" \) -print0)

    echo ""
    echo -e "${BLUE}Checking keydeck-config frontend files (Svelte)...${NC}"
    while IFS= read -r -d '' file; do
        if ! grep -q "SPDX-License-Identifier: AGPL-3.0-or-later" "$file"; then
            echo -e "${RED}✗ Missing AGPL header: $file${NC}"
            MISSING_FILES=$((MISSING_FILES + 1))
        fi
    done < <(find "$PROJECT_ROOT/keydeck-config/src" -type f -name "*.svelte" -print0)

    echo ""
    if [ $MISSING_FILES -eq 0 ]; then
        echo -e "${GREEN}✓ All source files have proper license headers${NC}"
        exit 0
    else
        echo -e "${RED}✗ Found $MISSING_FILES file(s) with missing license headers${NC}"
        exit 1
    fi
fi

# Handle install command
if [ "$1" = "install" ]; then
    echo "======================================"
    echo "KeyDeck Install"
    echo "======================================"
    echo ""

    INSTALL_DIR="$HOME/Works/System/bin"
    KEYDECK_BIN="$DIST_DIR/keydeck"
    KEYDECK_CONFIG_BIN="$DIST_DIR/keydeck-config"

    # Check if binaries exist
    if [ ! -f "$KEYDECK_BIN" ]; then
        echo -e "${RED}✗ keydeck binary not found at $KEYDECK_BIN${NC}"
        echo -e "${YELLOW}Run './build.sh daemon' or './build.sh app' first${NC}"
        exit 1
    fi

    if [ ! -f "$KEYDECK_CONFIG_BIN" ]; then
        echo -e "${RED}✗ keydeck-config binary not found at $KEYDECK_CONFIG_BIN${NC}"
        echo -e "${YELLOW}Run './build.sh ui' or './build.sh app' first${NC}"
        exit 1
    fi

    # Create install directory if it doesn't exist
    mkdir -p "$INSTALL_DIR"

    echo -e "${BLUE}Stopping keydeck service...${NC}"
    if systemctl --user is-active --quiet keydeck; then
        systemctl --user stop keydeck
        echo -e "${GREEN}✓ Service stopped${NC}"
    else
        echo -e "${YELLOW}Service is not running${NC}"
    fi
    echo ""

    echo -e "${BLUE}Copying binaries to $INSTALL_DIR...${NC}"
    cp -v "$KEYDECK_BIN" "$INSTALL_DIR/keydeck"
    cp -v "$KEYDECK_CONFIG_BIN" "$INSTALL_DIR/keydeck-config"
    chmod +x "$INSTALL_DIR/keydeck"
    chmod +x "$INSTALL_DIR/keydeck-config"
    echo -e "${GREEN}✓ Binaries copied${NC}"
    echo ""

    echo -e "${BLUE}Starting keydeck service...${NC}"
    systemctl --user start keydeck
    sleep 1

    if systemctl --user is-active --quiet keydeck; then
        echo -e "${GREEN}✓ Service started successfully${NC}"
    else
        echo -e "${RED}✗ Service failed to start${NC}"
        echo -e "${YELLOW}Check status with: systemctl --user status keydeck${NC}"
        exit 1
    fi

    echo ""
    echo -e "${GREEN}======================================"
    echo -e "Installation Complete!"
    echo -e "======================================${NC}"
    echo ""
    echo -e "${BLUE}Installed binaries:${NC}"
    ls -lh "$INSTALL_DIR/keydeck" "$INSTALL_DIR/keydeck-config"
    echo ""
    echo -e "${BLUE}Service status:${NC}"
    systemctl --user status keydeck --no-pager | head -10
    echo ""

    exit 0
fi

# Handle clean command
if [ "$1" = "clean" ]; then
    echo "======================================"
    echo "KeyDeck Clean"
    echo "======================================"
    echo ""
    echo -e "${YELLOW}This will remove all build artifacts and return the project to a fresh state.${NC}"
    echo -e "${YELLOW}The following will be deleted:${NC}"
    echo "  - target/"
    echo "  - dist/"
    echo "  - keydeck-config/node_modules/"
    echo "  - keydeck-config/.svelte-kit/"
    echo "  - keydeck-config/build/"
    echo "  - keydeck-config/src-tauri/target/"
    echo ""
    read -p "Are you sure? (y/N) " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        echo ""
        echo -e "${BLUE}Cleaning build artifacts...${NC}"

        rm -rf target
        rm -rf dist
        rm -rf keydeck-config/node_modules
        rm -rf keydeck-config/.svelte-kit
        rm -rf keydeck-config/build
        rm -rf keydeck-config/src-tauri/target

        echo -e "${GREEN}✓ Clean complete - project is now in fresh state${NC}"
        echo ""
        echo -e "${BLUE}To rebuild:${NC}"
        echo "  ./build.sh app"
    else
        echo "Clean cancelled"
    fi
    exit 0
fi

# Function to build Docker image if needed
build_image_if_needed() {
    if docker images | grep -q "^keydeck-builder-ubuntu\\s"; then
        echo -e "${BLUE}Using cached Ubuntu build image${NC}"
    else
        echo -e "${BLUE}Building Ubuntu Docker image...${NC}"
        docker build \
            --build-arg USER_ID=$USER_ID \
            --build-arg GROUP_ID=$GROUP_ID \
            -t keydeck-builder-ubuntu \
            -f "$DOCKER_DIR/ubuntu/Dockerfile" \
            "$DOCKER_DIR/ubuntu/"
    fi
}

# Function to clean build artifacts as root (handles permission issues)
clean_artifacts() {
    echo -e "${BLUE}Cleaning previous build artifacts...${NC}"
    docker run --rm \
        -v "$PROJECT_ROOT:/app" \
        -u root \
        keydeck-builder-ubuntu \
        bash -c "cd /app && rm -rf target keydeck-config/.svelte-kit keydeck-config/build keydeck-config/src-tauri/target keydeck-config/node_modules" || true
    echo -e "${GREEN}✓ Cleanup complete${NC}"
}

# Build daemon (keydeck CLI) binary
build_daemon() {
    echo -e "${YELLOW}=== Building keydeck daemon binary ===${NC}"
    echo -e "${BLUE}Running Rust build for keydeck...${NC}"
    docker run --rm \
        -v "$PROJECT_ROOT:/app" \
        -u $USER_ID:$GROUP_ID \
        keydeck-builder-ubuntu \
        bash -c "cd /app && cargo build --release --bins"

    if [ $? -ne 0 ]; then
        echo -e "${RED}✗ keydeck daemon build failed${NC}"
        exit 1
    fi

    echo -e "${GREEN}✓ keydeck daemon build complete${NC}"
    echo ""

    # Copy keydeck binary
    echo -e "${BLUE}Copying keydeck binary...${NC}"
    cp "$PROJECT_ROOT/target/release/keydeck" "$DIST_DIR/keydeck"
    echo -e "${GREEN}✓ keydeck binary copied to dist/keydeck${NC}"
    echo ""
}

# Build UI (keydeck-config) binary
build_ui() {
    echo -e "${YELLOW}=== Building keydeck-config UI binary ===${NC}"
    echo -e "${BLUE}Running Tauri build for keydeck-config...${NC}"
    docker run --rm \
        -v "$PROJECT_ROOT:/app" \
        -u $USER_ID:$GROUP_ID \
        keydeck-builder-ubuntu \
        bash -c "cd /app/keydeck-config && npm install && npm run tauri build -- --no-bundle"

    if [ $? -eq 0 ]; then
        echo -e "${GREEN}✓ keydeck-config UI build complete${NC}"
        echo ""

        # Copy binary
        echo -e "${BLUE}Copying keydeck-config binary...${NC}"
        cp "$PROJECT_ROOT/keydeck-config/src-tauri/target/release/keydeck-config" "$DIST_DIR/keydeck-config"
        echo -e "${GREEN}✓ keydeck-config binary copied to dist/keydeck-config${NC}"
    else
        echo -e "${RED}✗ keydeck-config UI build failed${NC}"
        exit 1
    fi
    echo ""
}

# Print final summary
print_summary() {
    echo ""
    echo -e "${GREEN}======================================"
    echo "Build Complete!"
    echo "======================================${NC}"
    echo ""
    echo -e "${BLUE}Binaries created in: $DIST_DIR${NC}"
    echo ""

    # List all binaries
    if [ -d "$DIST_DIR" ]; then
        ls -lh "$DIST_DIR"/ 2>/dev/null
    fi

    echo ""
    echo -e "${BLUE}Binary sizes:${NC}"
    if [ -f "$DIST_DIR/keydeck" ]; then
        echo -e "  keydeck:        $(du -h "$DIST_DIR/keydeck" | cut -f1)"
    fi
    if [ -f "$DIST_DIR/keydeck-config" ]; then
        echo -e "  keydeck-config: $(du -h "$DIST_DIR/keydeck-config" | cut -f1)"
    fi

    echo ""
    echo -e "${BLUE}To install:${NC}"
    if [ -f "$DIST_DIR/keydeck" ]; then
        echo "  sudo cp $DIST_DIR/keydeck /usr/local/bin/"
    fi
    if [ -f "$DIST_DIR/keydeck-config" ]; then
        echo "  sudo cp $DIST_DIR/keydeck-config /usr/local/bin/"
    fi
    echo ""
    echo -e "${BLUE}To rebuild Docker image from scratch:${NC}"
    echo "  docker rmi keydeck-builder-ubuntu"
    echo ""
}

# Build command starts here
if [ "$1" = "app" ] || [ "$1" = "daemon" ] || [ "$1" = "ui" ]; then
    echo "======================================"
    echo "KeyDeck Docker Binary Builder"
    echo "======================================"
    echo ""

    # Create dist directory
    mkdir -p "$DIST_DIR"

    # Get user ID for Docker
    USER_ID=$(id -u)
    GROUP_ID=$(id -g)

    echo -e "${BLUE}Project root: $PROJECT_ROOT${NC}"
    echo -e "${BLUE}Binaries output: $DIST_DIR${NC}"
    echo ""

    # Build Docker image
    echo -e "${YELLOW}=== Preparing build environment ===${NC}"
    echo ""
    build_image_if_needed
    clean_artifacts
    echo ""

    # Build based on command
    case "$1" in
        app)
            build_daemon
            build_ui
            ;;
        daemon)
            build_daemon
            ;;
        ui)
            build_ui
            ;;
    esac

    print_summary
fi
