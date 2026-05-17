#!/usr/bin/env bash
set -euo pipefail

usage() {
    echo "Usage: $0 <new-version>"
    echo "  e.g. $0 0.7.0"
    echo ""
    echo "Bumps the workspace version in Cargo.toml, commits, and creates a git tag."
    exit 1
}

[[ $# -ne 1 ]] && usage

NEW_VERSION="$1"

if ! [[ "$NEW_VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo "Error: version must be in semver format (e.g. 1.2.3)"
    exit 1
fi

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CARGO_TOML="$REPO_ROOT/Cargo.toml"

CURRENT_VERSION=$(grep -m1 '^version' "$CARGO_TOML" | sed 's/.*"\(.*\)".*/\1/')
echo "Current version: $CURRENT_VERSION"
echo "New version:     $NEW_VERSION"

if [[ "$CURRENT_VERSION" == "$NEW_VERSION" ]]; then
    echo "Error: new version is the same as current version"
    exit 1
fi

sed -i "s/^version = \"$CURRENT_VERSION\"/version = \"$NEW_VERSION\"/" "$CARGO_TOML"

cd "$REPO_ROOT"
cargo check --workspace 2>&1 | tail -1
echo "Version bumped successfully."

git add Cargo.toml
git commit -m "chore: bump version to v$NEW_VERSION"
git tag "v$NEW_VERSION"

echo ""
echo "Done! Created commit and tag v$NEW_VERSION."
echo "To publish: git push && git push --tags"
