#!/bin/bash
set -e

# Docker-based build script for KeyDeck packages
# Builds keydeck (CLI) and keydeck-config (UI) as separate packages
# Creates deb and rpm packages, then converts deb to Arch format

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo "======================================"
echo "KeyDeck Docker Package Builder"
echo "======================================"
echo ""

# Get directories
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
PACKAGES_DIR="$PROJECT_ROOT/packages"

# Create packages directory
mkdir -p "$PACKAGES_DIR"

# Get user ID for Docker
USER_ID=$(id -u)
GROUP_ID=$(id -g)

echo -e "${BLUE}Project root: $PROJECT_ROOT${NC}"
echo -e "${BLUE}Packages output: $PACKAGES_DIR${NC}"
echo ""

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
            -f "$SCRIPT_DIR/ubuntu/Dockerfile" \
            "$SCRIPT_DIR/ubuntu/"
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

# Function to convert deb to Arch package
convert_to_arch() {
    local deb_file=$1
    local package_name=$2

    if ! command -v debtap &> /dev/null; then
        echo -e "${YELLOW}Warning: debtap not found. Skipping Arch package for $package_name${NC}"
        echo -e "${YELLOW}Install with: yay -S debtap${NC}"
        return 1
    fi

    echo -e "${BLUE}Converting $package_name to Arch package...${NC}"

    cd "$PACKAGES_DIR"

    # Run debtap in quiet mode (skip all prompts)
    debtap -Q "$(basename "$deb_file")" 2>&1 | grep -v "^$" || true

    # Find the generated Arch package
    ARCH_FILE=$(find . -name "${package_name}*.pkg.tar.zst" -type f -newer "$deb_file" | head -1)

    if [ -n "$ARCH_FILE" ]; then
        ARCH_NAME=$(basename "$ARCH_FILE")

        # Fix dependencies in the Arch package
        echo -e "${BLUE}Fixing Arch package dependencies...${NC}"

        TEMP_DIR="${ARCH_NAME%.pkg.tar.zst}_temp"
        rm -rf "$TEMP_DIR"
        mkdir -p "$TEMP_DIR"

        # Extract
        tar -xf "$ARCH_NAME" -C "$TEMP_DIR"

        # Fix .PKGINFO
        if [ -f "$TEMP_DIR/.PKGINFO" ]; then
            sed -i \
                -e 's/^depend = gtk$/depend = gtk3/' \
                -e 's/^depend = libwebkit2gtk-4\.1-0$/depend = webkit2gtk-4.1/' \
                -e 's/^depend = libwebkit2gtk-4\.0-37$/depend = webkit2gtk/' \
                -e 's/^depend = libgtk-3-0$/depend = gtk3/' \
                -e 's/^depend = libglib2\.0-0$/depend = glib2/' \
                -e 's/^depend = libcairo2$/depend = cairo/' \
                -e 's/^depend = libpango-1\.0-0$/depend = pango/' \
                -e 's/^depend = libgdk-pixbuf-2\.0-0$/depend = gdk-pixbuf2/' \
                -e 's/^depend = libayatana-appindicator3-1$/depend = libappindicator-gtk3/' \
                -e 's/^depend = librsvg2-2$/depend = librsvg/' \
                -e 's/^depend = libudev1$/depend = systemd-libs/' \
                -e 's/^depend = libusb-1\.0-0$/depend = libusb/' \
                -e 's/^depend = libhidapi-hidraw0$/depend = hidapi/' \
                "$TEMP_DIR/.PKGINFO"

            # Repackage
            FIXED_NAME="${ARCH_NAME%.pkg.tar.zst}_fixed.pkg.tar.zst"
            cd "$TEMP_DIR"
            tar -c * .PKGINFO .MTREE 2>/dev/null | zstd -19 -T0 -q -o "../$FIXED_NAME"
            cd ..

            # Replace original with fixed version
            mv "$FIXED_NAME" "$ARCH_NAME"
            rm -rf "$TEMP_DIR"

            echo -e "${GREEN}✓ Arch package: $ARCH_NAME (dependencies fixed)${NC}"
        else
            echo -e "${YELLOW}Warning: Could not fix Arch package dependencies${NC}"
            echo -e "${GREEN}✓ Arch package: $ARCH_NAME${NC}"
        fi
    else
        echo -e "${RED}✗ Arch package conversion failed for $package_name${NC}"
        return 1
    fi
}

# Build Ubuntu image
echo -e "${YELLOW}=== Building packages ===${NC}"
echo ""
build_image_if_needed
clean_artifacts
echo ""

# Build keydeck CLI package
echo -e "${YELLOW}=== Building keydeck CLI package ===${NC}"
echo -e "${BLUE}Running Rust build for keydeck...${NC}"
docker run --rm \
    -v "$PROJECT_ROOT:/app" \
    -u $USER_ID:$GROUP_ID \
    keydeck-builder-ubuntu \
    bash -c "cd /app && cargo build --release --bins"

if [ $? -ne 0 ]; then
    echo -e "${RED}✗ keydeck CLI build failed${NC}"
    exit 1
fi

echo -e "${GREEN}✓ keydeck CLI build complete${NC}"
echo ""

# Create keydeck packages using cargo-deb and cargo-generate-rpm
echo -e "${BLUE}Creating keydeck deb package...${NC}"
docker run --rm \
    -v "$PROJECT_ROOT:/app" \
    -u $USER_ID:$GROUP_ID \
    keydeck-builder-ubuntu \
    bash -c "cd /app && cargo install cargo-deb 2>/dev/null || true && cargo deb --no-build"

if [ $? -eq 0 ]; then
    echo -e "${GREEN}✓ keydeck deb package created${NC}"
    # Copy deb to packages directory
    find "$PROJECT_ROOT/target/debian" -name "keydeck_*.deb" -exec cp {} "$PACKAGES_DIR/" \; 2>/dev/null
else
    echo -e "${RED}✗ keydeck deb package creation failed${NC}"
    exit 1
fi

echo -e "${BLUE}Creating keydeck rpm package...${NC}"
docker run --rm \
    -v "$PROJECT_ROOT:/app" \
    -u $USER_ID:$GROUP_ID \
    keydeck-builder-ubuntu \
    bash -c "cd /app && cargo install cargo-generate-rpm 2>/dev/null || true && cargo build --release && cargo generate-rpm"

if [ $? -eq 0 ]; then
    echo -e "${GREEN}✓ keydeck rpm package created${NC}"
    # Copy rpm to packages directory
    find "$PROJECT_ROOT/target/generate-rpm" -name "keydeck-*.rpm" -exec cp {} "$PACKAGES_DIR/" \; 2>/dev/null
else
    echo -e "${RED}✗ keydeck rpm package creation failed${NC}"
    exit 1
fi

echo ""

# Build keydeck-config UI package
echo -e "${YELLOW}=== Building keydeck-config UI package ===${NC}"
echo -e "${BLUE}Running Tauri build for keydeck-config...${NC}"
docker run --rm \
    -v "$PROJECT_ROOT:/app" \
    -u $USER_ID:$GROUP_ID \
    keydeck-builder-ubuntu \
    bash -c "cd /app/keydeck-config && npm install && npm run tauri build -- --bundles deb,rpm"

if [ $? -eq 0 ]; then
    echo -e "${GREEN}✓ keydeck-config UI build complete${NC}"

    # Copy deb and rpm to packages directory
    find "$PROJECT_ROOT/keydeck-config/src-tauri/target/release/bundle/deb" -name "*.deb" -exec cp {} "$PACKAGES_DIR/" \; 2>/dev/null
    find "$PROJECT_ROOT/keydeck-config/src-tauri/target/release/bundle/rpm" -name "*.rpm" -exec cp {} "$PACKAGES_DIR/" \; 2>/dev/null
else
    echo -e "${RED}✗ keydeck-config UI build failed${NC}"
    exit 1
fi

echo ""

# Convert deb packages to Arch format
echo -e "${YELLOW}=== Converting packages to Arch format ===${NC}"
echo ""

# Find the deb packages
KEYDECK_DEB=$(find "$PACKAGES_DIR" -name "keydeck_*.deb" -type f | head -1)
KEYDECK_CONFIG_DEB=$(find "$PACKAGES_DIR" -name "keydeck-config_*.deb" -type f | head -1)

echo -e "${BLUE}Found packages:${NC}"
[ -n "$KEYDECK_DEB" ] && echo -e "  keydeck: $(basename "$KEYDECK_DEB")"
[ -n "$KEYDECK_CONFIG_DEB" ] && echo -e "  keydeck-config: $(basename "$KEYDECK_CONFIG_DEB")"
echo ""

if [ -n "$KEYDECK_DEB" ]; then
    convert_to_arch "$KEYDECK_DEB" "keydeck"
else
    echo -e "${YELLOW}Warning: keydeck deb package not found${NC}"
fi

if [ -n "$KEYDECK_CONFIG_DEB" ]; then
    convert_to_arch "$KEYDECK_CONFIG_DEB" "keydeck-config"
else
    echo -e "${YELLOW}Warning: keydeck-config deb package not found${NC}"
fi

echo ""
echo -e "${GREEN}======================================"
echo "Build Complete!"
echo "======================================${NC}"
echo ""
echo -e "${BLUE}Packages created in: $PACKAGES_DIR${NC}"
echo ""

# List all packages
if [ -d "$PACKAGES_DIR" ]; then
    ls -lh "$PACKAGES_DIR"/ 2>/dev/null
fi

echo ""
echo -e "${BLUE}Installation commands:${NC}"
echo ""

KEYDECK_DEB=$(find "$PACKAGES_DIR" -name "keydeck_*.deb" 2>/dev/null | head -1)
KEYDECK_RPM=$(find "$PACKAGES_DIR" -name "keydeck-*.rpm" 2>/dev/null | head -1)
KEYDECK_ARCH=$(find "$PACKAGES_DIR" -name "keydeck-*.pkg.tar.zst" 2>/dev/null | head -1)

KEYDECK_CONFIG_DEB=$(find "$PACKAGES_DIR" -name "*keydeck*config*.deb" -o -name "*key-deck-configuration*.deb" 2>/dev/null | head -1)
KEYDECK_CONFIG_RPM=$(find "$PACKAGES_DIR" -name "*keydeck*config*.rpm" -o -name "*key-deck-configuration*.rpm" 2>/dev/null | head -1)
KEYDECK_CONFIG_ARCH=$(find "$PACKAGES_DIR" -name "*keydeck*config*.pkg.tar.zst" -o -name "*key-deck-configuration*.pkg.tar.zst" 2>/dev/null | head -1)

if [ -n "$KEYDECK_DEB" ] && [ -n "$KEYDECK_CONFIG_DEB" ]; then
    echo -e "${YELLOW}Debian/Ubuntu:${NC}"
    echo "  sudo dpkg -i $(basename "$KEYDECK_DEB") $(basename "$KEYDECK_CONFIG_DEB")"
    echo ""
fi

if [ -n "$KEYDECK_RPM" ] && [ -n "$KEYDECK_CONFIG_RPM" ]; then
    echo -e "${YELLOW}Fedora/RHEL:${NC}"
    echo "  sudo rpm -i $(basename "$KEYDECK_RPM") $(basename "$KEYDECK_CONFIG_RPM")"
    echo ""
fi

if [ -n "$KEYDECK_ARCH" ] && [ -n "$KEYDECK_CONFIG_ARCH" ]; then
    echo -e "${YELLOW}Arch/Manjaro:${NC}"
    echo "  sudo pacman -U $(basename "$KEYDECK_ARCH") $(basename "$KEYDECK_CONFIG_ARCH")"
    echo ""
fi

echo -e "${BLUE}To rebuild images from scratch:${NC}"
echo "  docker rmi keydeck-builder-ubuntu"
echo ""
