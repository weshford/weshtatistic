#!/usr/bin/env bash
# Update and regenerate the committed subsetted fallback fonts in
# crates/weshtatistic-gui/assets/fonts/ against the latest translation strings.
#
# Requires: python3 with fonttools installed (`pip install fonttools` or
# `pacman -S python-fonttools`).
set -euo pipefail

cd "$(dirname "$0")/.."

echo "==> 1. Ensuring raw Noto sources are downloaded..."
./scripts/fetch_fonts.sh

echo "==> 2. Generating fluent charset corpus via cargo check..."
cargo check -p weshtatistic-gui --quiet

echo "==> 3. Running font subsetter..."
python3 scripts/subset_fonts.py

echo "==> 4. Verifying generated fonts with cargo test..."
cargo test -p weshtatistic-gui --test weshtatistic_gui -- subset_fonts || cargo test -p weshtatistic-gui -- subset_fonts

echo "==> Done! Subsetted fonts updated in crates/weshtatistic-gui/assets/fonts/."
echo "    Remember to stage and commit them: git add crates/weshtatistic-gui/assets/fonts/"
