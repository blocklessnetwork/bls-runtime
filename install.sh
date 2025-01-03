#!/bin/bash

# Set the installation directory to the user's bin directory
INSTALL_DIR="$HOME/bin"

# Create the bin directory if it doesn't exist
mkdir -p $INSTALL_DIR

# Determine the operating system and architecture
OS=$(uname -s)
ARCH=$(uname -m)

# Map architectures to download names
case $ARCH in
    "x86_64")
        ARCH_NAME="x86_64"
        ;;
    "aarch64"|"arm64")
        ARCH_NAME="aarch64"
        ;;
    *)
        echo "Unsupported architecture: $ARCH"
        exit 1
        ;;
esac

# Determine the download URL based on the operating system
case $OS in
    "Linux")
        if [[ "$ARCH" == "x86_64" ]]; then
            URL="https://github.com/blocklessnetwork/bls-runtime/releases/download/v0.4.0/blockless-runtime.linux-latest.x86_64.tar.gz"
        elif [[ "$ARCH" == "aarch64" ]]; then
            URL="https://github.com/blocklessnetwork/bls-runtime/releases/download/v0.4.0/blockless-runtime.linux-latest.aarch64.tar.gz"
        fi
        ;;
    "Darwin")
        if [[ "$ARCH" == "x86_64" ]]; then
            URL="https://github.com/blocklessnetwork/bls-runtime/releases/download/v0.4.0/blockless-runtime.macos-latest.x86_64.tar.gz"
        elif [[ "$ARCH" == "aarch64" || "$ARCH" == "arm64" ]]; then
            URL="https://github.com/blocklessnetwork/bls-runtime/releases/download/v0.4.0/blockless-runtime.macos-latest.aarch64.tar.gz"
        fi
        ;;
    "WindowsNT")
        if [[ "$ARCH" == "x86_64" ]]; then
            URL="https://github.com/blocklessnetwork/bls-runtime/releases/download/v0.4.0/blockless-runtime.windows-latest.x86_64.tar.gz"
        fi
        ;;
    *)
        echo "Unsupported OS: $OS"
        exit 1
        ;;
esac

# Download the binary
echo "Downloading Blockless Runtime from $URL..."
curl -L $URL -o /tmp/blockless-runtime.tar.gz

# Extract the downloaded tar.gz file
echo "Extracting Blockless Runtime..."
tar -xzf /tmp/blockless-runtime.tar.gz -C /tmp

# Move the binary to the user's bin directory
echo "Installing Blockless Runtime to $INSTALL_DIR..."
mv /tmp/bls-runtime $INSTALL_DIR

# Make sure the binary is executable
chmod +x $INSTALL_DIR/bls-runtime

# Clean up
rm /tmp/blockless-runtime.tar.gz

# Add ~/bin to PATH if not already added
if [[ ":$PATH:" != *":$HOME/bin:"* ]]; then
    echo "Adding $HOME/bin to PATH in your shell profile..."
    echo 'export PATH="$HOME/bin:$PATH"' >> ~/.bash_profile || ~/.zshrc
    source ~/.bash_profile || source ~/.zshrc
fi

# Verify the installation
echo "Installation complete!"
bls-runtime --version
