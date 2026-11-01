# Horizon iOS: experimental test port

This adds iOS startup, touch input and a macOS packaging script to the actual Rust
client. It is not a released or device-verified port. Requires iOS 16+, an arm64
Metal device, and macOS with full Xcode selected (`xcode-select -s /Applications/Xcode.app`).
Rust stable must be installed. The first test deliberately disables audio and
hardware controllers (`--no-default-features`).

## Build and run

1. Prepare resources from **your own Minecraft 26.3 installation** on a desktop,
   following [the root README](../../README.md#build-from-source). Java exporters
   run on the desktop, never on the phone. Locate the prepared directory containing
   `horizon-resources.json`; copy it to your Mac if necessary.
2. On an Apple Silicon Mac, boot an iPhone simulator in Xcode, then run from the
   repository root:

   ```sh
   packaging/ios/build.sh simulator /absolute/path/to/prepared/resources
   xcrun simctl install booted dist/ios/simulator/Payload/Horizon.app
   xcrun simctl launch --console booted dev.horizon.ios-test
   ```

   For a physical iPhone:

   ```sh
   packaging/ios/build.sh device /absolute/path/to/prepared/resources
   ```

   Sign `dist/ios/device/Horizon-unsigned.ipa` using your own Apple development
   identity/provisioning profile or a sideloading tool, then install. The script
   does not manage Apple accounts or signing. Never publish bundles containing
   Minecraft resources.

The app opens `iOS Test` in creative mode with simple test terrain and render
distance 2, using the normal integrated server. Saves, options, caches and logs go
to the sandbox's `Documents/Horizon-Test`, available through Files file sharing.
The resource bundle stays read-only. These test saves are separate from desktop
worlds. Simulator command-line game arguments override the test defaults.

## Touch controls

- Left pad: fixed joystick; slide away from the centre to move, including diagonals.
- Swipe in free space: look around. Other fingers can move, jump or act simultaneously.
- `+` on the right: jump; `_`: sneak; `X`: attack/mine; `O`: place/use.
- Tap a slot in the existing nine-slot hotbar to switch items; `...` opens inventory.
- `||` opens pause. Menus and inventory accept taps and drags.
- Top-left `+` / `#` switches joystick / D-pad. Settings apply for this session.

Finger cancellation, focus loss and resizing release touch controls; opening a
screen releases held gameplay controls. Layout uses GUI coordinates with screen
edge margins. Try `HORIZON_TOUCH=1` on desktop to display the overlay for layout
inspection (a mouse is not a multi-touch emulator).

## Verification and limits

The dependency-free touch state machine can be tested without an Apple SDK:

```sh
rustc --edition=2024 --test crates/horizon-client-input/src/touch.rs -o /tmp/touch-tests
/tmp/touch-tests
```

`.github/workflows/ios.yml` builds the device target on macOS; it
does not execute the app or distribute proprietary resources. Device testing is
still required for Metal rendering, frame rate, memory pressure, safe-area layout,
background/resume and save persistence. This first pass has no native iOS keyboard,
system clipboard, adaptive safe-area layout or background save integration. Save
and quit from the pause menu before leaving the app; iOS may terminate background
apps. It is not ready for App Store/TestFlight distribution.

## GitHub Actions IPA

The `iOS test IPA` workflow cross-compiles an optimized arm64 device IPA on an Intel
macOS runner and uploads
`Horizon-iOS-unsigned`. It runs on pushes to `codex/ios-test-port`, or manually.
The IPA is **unsigned and contains no Minecraft resources**. Sign it with your
own Apple development profile or sideloading tool before installing.

After first launch, open the app's shared Documents folder through Files or
Finder, and copy the contents of your prepared resource folder into
`Horizon-Test/resources`. `horizon-resources.json` must be directly inside that
folder, alongside the other prepared files. Fully close and reopen the app to
load the resources. Until then, the app shows the missing-resource screen.
The regular desktop launcher download message on that screen does not apply
to this experimental iOS build; use the import procedure above.

Local builds without resources use:

```sh
packaging/ios/build.sh device --without-resources
```

## Startup stack fix (0.1.1)

Version 0.1.0 was unoptimized and overflowed the iPhone main thread stack while
creating the missing-resources screen. Device builds now use the optimized
`fast` profile. Renderer initialization futures and the missing-screen state live
on the heap, and the startup callback avoids a large intermediate renderer tuple.
The workflow also starts a separate, disposable iOS simulator without game assets
and captures its first frame. A hosted simulator with no Metal device reports an
explicit rendering skip; it is not evidence of rendering on a physical iPhone.
Simulator builds select the architecture of the Mac host automatically.
