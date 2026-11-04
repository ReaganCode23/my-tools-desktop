#!/bin/sh
set -eu
SOURCE_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
APP_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/mytools"
APPS_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/applications"
DESKTOP_DIR=$(xdg-user-dir DESKTOP 2>/dev/null || printf '%s/Desktop' "$HOME")
mkdir -p "$APP_DIR" "$APPS_DIR"
cp "$SOURCE_DIR/mytools" "$APP_DIR/mytools"
chmod 755 "$APP_DIR/mytools"
cp "$SOURCE_DIR/mytools.svg" "$APP_DIR/mytools.svg"
LAUNCHER="$APPS_DIR/com.reagancode23.mytools.desktop"
cat > "$LAUNCHER" <<ENTRY
[Desktop Entry]
Type=Application
Name=MyTools
Comment=Your private, offline desktop toolkit
Exec="$APP_DIR/mytools"
Icon=$APP_DIR/mytools.svg
Terminal=false
Categories=Utility;Office;
StartupWMClass=mytools
ENTRY
chmod 755 "$LAUNCHER"
if [ -d "$DESKTOP_DIR" ]; then
 cp "$LAUNCHER" "$DESKTOP_DIR/MyTools.desktop"
 chmod 755 "$DESKTOP_DIR/MyTools.desktop"
 gio set "$DESKTOP_DIR/MyTools.desktop" metadata::trusted true 2>/dev/null || true
fi
update-desktop-database "$APPS_DIR" 2>/dev/null || true
printf 'MyTools is installed. Open MyTools on your desktop or in your app menu.\n'
