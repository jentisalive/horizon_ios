#!/bin/bash
# Build on macOS with Xcode selected. No Mojang files are downloaded or published.
set -euo pipefail
cd "$(dirname "$0")/../.."
if [[ $(uname -s) != Darwin ]]; then
    echo 'Requires macOS and Xcode (iOS SDK).' >&2; exit 1
fi
mode=${1:-simulator}
resources=${2:-.local/horizon-client/26.3}
case "$mode" in
    simulator) target=aarch64-apple-ios-sim; sdk=iphonesimulator ;;
    device) target=aarch64-apple-ios; sdk=iphoneos ;;
    *) echo 'Usage: build.sh simulator|device /path/to/prepared/resources' >&2; exit 1 ;;
esac
if [[ "$resources" != --without-resources && ! -f "$resources/horizon-resources.json" ]]; then
    echo 'Prepare resources from your own Minecraft copy first; see README.md.' >&2; exit 1
fi
if [[ "$resources" != --without-resources ]]; then resources=$(cd "$resources" && pwd); fi
export IPHONEOS_DEPLOYMENT_TARGET=16.0
export SDKROOT
SDKROOT=$(xcrun --sdk "$sdk" --show-sdk-path)
rustup target add "$target"
profile=${HORIZON_IOS_PROFILE:-fast}
profile_dir=$profile
if [[ "$profile" == dev ]]; then profile_dir=debug; fi
cargo build --locked --profile "$profile" -p horizon-client --no-default-features --target "$target"
out="$PWD/dist/ios/$mode"
app="$out/Payload/Horizon.app"
mkdir -p "$app"
cp "target/$target/$profile_dir/horizon-client" "$app/horizon-client"
cp packaging/ios/Info.plist "$app/Info.plist"
# Replace resources on repeat builds so deleted files cannot linger.
if [[ "$resources" == --without-resources ]]; then
    echo 'Building without game resources; import them into Documents/Horizon-Test/resources before playing.'
else
    rsync -a --delete "$resources/" "$app/resources/"
fi
if [[ "$mode" == simulator ]]; then
    codesign --force --sign - "$app"
    echo "Built $app"
    echo "Install: xcrun simctl install booted '$app'"
    echo 'Launch: xcrun simctl launch booted dev.horizon.ios-test'
else
    # Unsigned IPA for signing with a personal development profile/sideloading tool.
    (cd "$out" && zip -qry Horizon-unsigned.ipa Payload)
    echo "Built $out/Horizon-unsigned.ipa (requires signing before installation)"
fi
