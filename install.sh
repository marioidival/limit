#!/bin/bash
#
# Limit CLI Installer
# https://github.com/marioidival/limit
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/marioidival/limit/main/install.sh | bash
#

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Configuration
GITHUB_REPO="marioidival/limit"
GITHUB_API_URL="https://api.github.com/repos/${GITHUB_REPO}/releases/latest"
INSTALL_DIR="${HOME}/.local/bin"
BINARY_NAME="limit"

# Print functions
info() { echo -e "${BLUE}➜${NC} $1"; }
success() { echo -e "${GREEN}✓${NC} $1"; }
warn() { echo -e "${YELLOW}!${NC} $1"; }
error() { echo -e "${RED}✗${NC} $1"; exit 1; }

# Banner
echo ""
echo -e "${BLUE}╔═══════════════════════════════════════╗${NC}"
echo -e "${BLUE}║         Limit CLI Installer           ║${NC}"
echo -e "${BLUE}║   Your AI pair programmer in terminal  ║${NC}"
echo -e "${BLUE}╚═══════════════════════════════════════╝${NC}"
echo ""

# Detect OS and Architecture
detect_platform() {
    OS=$(uname -s | tr '[:upper:]' '[:lower:]')
    ARCH=$(uname -m)
    
    case "$OS" in
        linux)  OS="linux" ;;
        darwin) OS="macos" ;;
        *)      error "Unsupported OS: $OS" ;;
    esac
    
    case "$ARCH" in
        x86_64|amd64)   ARCH="x86_64" ;;
        aarch64|arm64)  ARCH="aarch64" ;;
        *)              error "Unsupported architecture: $ARCH" ;;
    esac
    
    info "Detected: ${OS}/${ARCH}"
}

# Get latest release version
get_latest_version() {
    info "Fetching latest release..."
    
    if command -v curl &> /dev/null; then
        VERSION=$(curl -s "${GITHUB_API_URL}" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')
    elif command -v wget &> /dev/null; then
        VERSION=$(wget -qO- "${GITHUB_API_URL}" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')
    else
        error "curl or wget is required"
    fi
    
    if [ -z "$VERSION" ]; then
        error "Failed to get latest version"
    fi
    
    success "Latest version: ${VERSION}"
}

# Download and install
install_binary() {
    local DOWNLOAD_URL="https://github.com/${GITHUB_REPO}/releases/download/${VERSION}/limit-${OS}-${ARCH}.tar.gz"
    local TEMP_DIR=$(mktemp -d)
    local TEMP_FILE="${TEMP_DIR}/limit.tar.gz"
    
    # Cleanup on exit
    cleanup() {
        rm -rf "$TEMP_DIR"
    }
    trap cleanup EXIT
    
    info "Downloading Limit ${VERSION} for ${OS}-${ARCH}..."
    
    if command -v curl &> /dev/null; then
        curl -fsSL -o "$TEMP_FILE" "$DOWNLOAD_URL" || {
            error "Download failed. Release may not exist for ${OS}-${ARCH}.\n\nTry building from source:\n  git clone https://github.com/${GITHUB_REPO}\n  cd limit && cargo build --release"
        }
    elif command -v wget &> /dev/null; then
        wget -q -O "$TEMP_FILE" "$DOWNLOAD_URL" || {
            error "Download failed. Release may not exist for ${OS}-${ARCH}.\n\nTry building from source:\n  git clone https://github.com/${GITHUB_REPO}\n  cd limit && cargo build --release"
        }
    fi
    
    success "Download complete"
    
    info "Extracting..."
    tar -xzf "$TEMP_FILE" -C "$TEMP_DIR"
    
    # Create install directory
    mkdir -p "$INSTALL_DIR"
    
    # Install binary
    info "Installing to ${INSTALL_DIR}..."
    mv "${TEMP_DIR}/${BINARY_NAME}" "${INSTALL_DIR}/${BINARY_NAME}"
    chmod +x "${INSTALL_DIR}/${BINARY_NAME}"
    
    success "Installed: ${INSTALL_DIR}/${BINARY_NAME}"
}

# Check PATH
check_path() {
    if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
        warn "${INSTALL_DIR} is not in your PATH"
        echo ""
        echo -e "${YELLOW}Add this to your shell (~/.bashrc, ~/.zshrc, etc.):${NC}"
        echo ""
        echo "    export PATH=\"\$HOME/.local/bin:\$PATH\""
        echo ""
        echo -e "${YELLOW}Then run:${NC}"
        echo ""
        echo "    source ~/.bashrc  # or ~/.zshrc"
        echo ""
    fi
}

# Main
main() {
    detect_platform
    get_latest_version
    install_binary
    check_path
    
    # Success message
    echo ""
    echo -e "${GREEN}═══════════════════════════════════════${NC}"
    echo -e "${GREEN}  Limit installed successfully!        ${NC}"
    echo -e "${GREEN}═══════════════════════════════════════${NC}"
    echo ""
    echo "Quick start:"
    echo ""
    echo "  1. Configure API key:"
    echo "     echo 'provider = \"anthropic\"' > ~/.limit/config.toml"
    echo "     export ANTHROPIC_API_KEY=\"your-key-here\""
    echo ""
    echo "  2. Run:"
    echo "     limit"
    echo ""
}

main "$@"
