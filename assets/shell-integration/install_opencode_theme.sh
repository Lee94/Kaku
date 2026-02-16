#!/bin/bash
# Kaku - OpenCode Theme Installation Script
# Installs a Kaku-matching color theme for OpenCode

set -euo pipefail

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BOLD='\033[1m'
NC='\033[0m'

OPENCODE_DIR="$HOME/.config/opencode"
THEMES_DIR="$OPENCODE_DIR/themes"
CONFIG_FILE="$OPENCODE_DIR/opencode.json"
THEME_FILE="$THEMES_DIR/wezterm-match.json"

echo -e "${BOLD}OpenCode Theme Setup${NC}"
echo -e "${NC}Kaku-matching color palette for OpenCode${NC}"

if [[ -f "$CONFIG_FILE" ]]; then
    read -p "OpenCode config already exists. Overwrite with Kaku theme? [Y/n] " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Nn]$ ]]; then
        echo -e "${YELLOW}Skipped${NC}"
        exit 0
    fi
fi

mkdir -p "$OPENCODE_DIR"
mkdir -p "$THEMES_DIR"

echo -n "  Installing OpenCode theme... "
cat > "$THEME_FILE" << 'THEME_EOF'
{
  "$schema": "https://opencode.ai/theme.json",
  "defs": {
    "bg": "#15141b",
    "panel": "#15141b",
    "element": "#1f1d28",
    "text": "#edecee",
    "muted": "#6b6b6b",
    "primary": "#a277ff",
    "secondary": "#61ffca",
    "accent": "#ffca85",
    "error": "#ff6767",
    "warning": "#ffca85",
    "success": "#61ffca",
    "info": "#a277ff",
    "border": "#15141b",
    "border_active": "#29263c",
    "border_subtle": "#15141b"
  },
  "theme": {
    "primary": "primary",
    "secondary": "secondary",
    "accent": "accent",
    "error": "error",
    "warning": "warning",
    "success": "success",
    "info": "info",
    "text": "text",
    "textMuted": "muted",
    "background": "bg",
    "backgroundPanel": "panel",
    "backgroundElement": "element",
    "border": "border",
    "borderActive": "border_active",
    "borderSubtle": "border_subtle",
    "diffAdded": "success",
    "diffRemoved": "error",
    "diffContext": "muted",
    "diffHunkHeader": "primary",
    "diffHighlightAdded": "success",
    "diffHighlightRemoved": "error",
    "diffAddedBg": "#1b2a24",
    "diffRemovedBg": "#2a1b20",
    "diffContextBg": "bg",
    "diffLineNumber": "muted",
    "diffAddedLineNumberBg": "#1b2a24",
    "diffRemovedLineNumberBg": "#2a1b20",
    "markdownText": "text",
    "markdownHeading": "primary",
    "markdownLink": "info",
    "markdownLinkText": "primary",
    "markdownCode": "accent",
    "markdownBlockQuote": "muted",
    "markdownEmph": "accent",
    "markdownStrong": "secondary",
    "markdownHorizontalRule": "muted",
    "markdownListItem": "primary",
    "markdownListEnumeration": "accent",
    "markdownImage": "info",
    "markdownImageText": "primary",
    "markdownCodeBlock": "text",
    "syntaxComment": "muted",
    "syntaxKeyword": "primary",
    "syntaxFunction": "secondary",
    "syntaxVariable": "text",
    "syntaxString": "success",
    "syntaxNumber": "accent",
    "syntaxType": "info",
    "syntaxOperator": "primary",
    "syntaxPunctuation": "text"
  }
}
THEME_EOF
echo -e "${GREEN}done ✅${NC}"

echo -n "  Writing OpenCode config... "
cat > "$CONFIG_FILE" << 'CONFIG_EOF'
{
  "theme": "wezterm-match"
}
CONFIG_EOF
echo -e "${GREEN}done ✅${NC}"

echo ""
echo -e "${GREEN}${BOLD}✓ OpenCode theme configured successfully!${NC}"
echo ""
