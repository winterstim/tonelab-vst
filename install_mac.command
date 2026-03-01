#!/bin/bash
echo "========================================="
echo "      Tonelab VST Installer (macOS)      "
echo "========================================="
echo "This script will copy Tonelab to your VST3 folder"
echo "and remove the macOS security 'quarantine' flag."
echo ""
echo "You will be prompted to enter your Mac login password."
echo ""

# Get the directory where the script is located
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PLUGIN_SRC="$SCRIPT_DIR/tonelab_vst.vst3"
DEST_DIR="/Library/Audio/Plug-Ins/VST3/"
PLUGIN_DEST="$DEST_DIR/tonelab_vst.vst3"

if [ ! -d "$PLUGIN_SRC" ] && [ ! -f "$PLUGIN_SRC" ]; then
    echo "Error: tonelab_vst.vst3 not found in $(dirname "$0")."
    echo "Please extract the entire zip archive before running this script."
    echo "Press any key to exit..."
    read -n 1 -s
    exit 1
fi

sudo mkdir -p "$DEST_DIR" || { echo "Failed to create directory $DEST_DIR"; exit 1; }

echo "Copying plugin to $DEST_DIR..."
# Remove if exists to do a clean install
sudo rm -rf "$PLUGIN_DEST"
sudo cp -R "$PLUGIN_SRC" "$DEST_DIR" || { echo "Failed to copy plugin."; exit 1; }

echo "Removing quarantine flag (Gatekeeper bypass)..."
sudo xattr -rd com.apple.quarantine "$PLUGIN_DEST" || { echo "Warning: Failed to remove quarantine flag. You might need to allow it manually in System Settings."; }

echo ""
echo "========================================="
echo " Installation complete! "
echo " You can now use Tonelab in your DAW."
echo "========================================="
echo "Press any key to close this window..."
read -n 1 -s
echo ""
