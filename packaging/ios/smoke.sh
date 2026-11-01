#!/bin/bash
# Regress the device crash path without bundling any Minecraft resources.
set -euo pipefail
cd "$(dirname "$0")/../.."
out="$PWD/dist/ios/simulator-smoke"
mkdir -p "$out"
scratch=$(mktemp -d)
xcrun simctl list runtimes -j > "$scratch/runtimes.json"
xcrun simctl list devicetypes -j > "$scratch/devices.json"
runtime=$(python3 -c 'import json,sys; r=[r["identifier"] for r in json.load(open(sys.argv[1]))["runtimes"] if r.get("isAvailable") and "iOS" in r["identifier"]]; assert r,"No iOS simulator runtime installed"; print(r[-1])' "$scratch/runtimes.json")
device_type=$(python3 -c 'import json,sys; d=[d["identifier"] for d in json.load(open(sys.argv[1]))["devicetypes"] if d["name"] == "iPhone 16"]; assert d,"No iPhone 16 simulator type"; print(d[0])' "$scratch/devices.json")
udid=$(xcrun simctl create "Horizon regression $(uuidgen)" "$device_type" "$runtime")
cleanup() {
    # A failed application must not hang the harness while it shuts down.
    python3 - "$udid" <<'PY_CLEANUP'
import subprocess, sys
for command in [
    ["xcrun", "simctl", "terminate", sys.argv[1], "dev.horizon.ios-test"],
    ["xcrun", "simctl", "shutdown", sys.argv[1]],
    ["xcrun", "simctl", "delete", sys.argv[1]],
]:
    try:
        subprocess.run(command, timeout=20, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    except subprocess.TimeoutExpired:
        pass
PY_CLEANUP
    rm -rf "$scratch"
}
trap cleanup EXIT
xcrun simctl boot "$udid"
xcrun simctl bootstatus "$udid" -b
xcrun simctl install "$udid" dist/ios/simulator/Payload/Horizon.app
# All game and launcher data goes into this disposable simulator/scratch home.
export SIMCTL_CHILD_HORIZON_HOME="$scratch/game"
export SIMCTL_CHILD_HORIZON_LAUNCHER_HOME="$scratch/launcher"
export SIMCTL_CHILD_XDG_DATA_HOME="$scratch/data"
export SIMCTL_CHILD_XDG_CONFIG_HOME="$scratch/config"
export SIMCTL_CHILD_HORIZON_LAUNCHER_NO_SELF_UPDATE=1
export SIMCTL_CHILD_HORIZON_NO_AUDIO=1
xcrun simctl launch --console "$udid" dev.horizon.ios-test > "$out/console.log" 2>&1 &
launcher=$!
for ((attempt=0; attempt<120; attempt++)); do
    if grep -Eq 'SIGSEGV|Segmentation fault|stack overflow|EXC_BAD_ACCESS|exited due to signal|panicked at|wgpu error: Validation Error' "$out/console.log"; then
        cat "$out/console.log"
        exit 1
    fi
    if grep -Eq 'presenting the first frame' "$out/console.log"; then
        xcrun simctl io "$udid" screenshot "$out/missing-resources.png"
        echo 'PASS: missing-resources screen presented its first frame.' | tee "$out/result.txt"
        if [[ -n ${GITHUB_STEP_SUMMARY:-} ]]; then cat "$out/result.txt" >> "$GITHUB_STEP_SUMMARY"; fi
        exit 0
    fi
    if grep -Eq 'No Metal graphics device found' "$out/console.log"; then
        echo 'SKIP rendering: this hosted simulator has no Metal device. Startup reached GPU discovery without the reported stack fault.' | tee "$out/result.txt"
        if [[ -n ${GITHUB_STEP_SUMMARY:-} ]]; then cat "$out/result.txt" >> "$GITHUB_STEP_SUMMARY"; fi
        exit 0
    fi
    if ! kill -0 "$launcher" 2>/dev/null; then
        cat "$out/console.log"
        echo 'App exited before presenting a frame.' >&2
        exit 1
    fi
    sleep 1
done
cat "$out/console.log"
echo 'Timed out waiting for the first frame.' >&2
exit 1
