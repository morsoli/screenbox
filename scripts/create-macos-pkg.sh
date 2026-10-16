#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/.."

target="${1:?Usage: create-macos-pkg.sh <rust-target> <release-tag> <arch> }"
release_tag="${2:?Usage: create-macos-pkg.sh <rust-target> <release-tag> <arch> }"
arch="${3:?Usage: create-macos-pkg.sh <rust-target> <release-tag> <arch> }"
app_path="src-tauri/target/${target}/release/bundle/macos/Screenbox.app"
if [[ ! -d "$app_path" || ! -f "$app_path/Contents/Resources/icon.icns" ]]; then
  app_path="src-tauri/target/release/bundle/macos/Screenbox.app"
fi
app_version="$(node -p 'require("./package.json").version')"
output="Screenbox-${release_tag}-macOS-${arch}.pkg"
temp_root="$(mktemp -d)"
stage="$temp_root/root"
component_plist="$temp_root/components.plist"
trap 'rm -rf "$temp_root"' EXIT

[[ -d "$app_path" ]] || { echo "App bundle not found: $app_path" >&2; exit 1; }
[[ -f "$app_path/Contents/Resources/icon.icns" ]] || { echo "App icon is missing: $app_path/Contents/Resources/icon.icns" >&2; exit 1; }
icon_name="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleIconFile' "$app_path/Contents/Info.plist")"
[[ "$icon_name" == "icon.icns" ]] || { echo "Unexpected CFBundleIconFile: $icon_name" >&2; exit 1; }
mkdir -p "$stage/Applications"
ditto --norsrc "$app_path" "$stage/Applications/Screenbox.app"
pkgbuild --analyze --root "$stage" "$component_plist"
/usr/libexec/PlistBuddy -c 'Set :0:BundleIsRelocatable false' "$component_plist"
pkgbuild \
  --root "$stage" \
  --component-plist "$component_plist" \
  --install-location / \
  --identifier com.morso.screenbox.pkg \
  --version "$app_version" \
  "$output"
pkgutil --expand "$output" "$temp_root/expanded"
package_info="$temp_root/expanded/PackageInfo"
grep -q 'relocatable="false"' "$package_info" || { echo "Installer package allows app relocation: $package_info" >&2; exit 1; }
grep -q 'install-location="/"' "$package_info" || { echo "Installer location is not the filesystem root: $package_info" >&2; exit 1; }
grep -q 'path="./Applications/Screenbox.app"' "$package_info" || { echo "Installer payload does not target /Applications/Screenbox.app: $package_info" >&2; exit 1; }
echo "Created $output"
