# Mozui iOS Test Host

This is the fastest native phone-test scaffold for the current `mozui` iOS backend.

## Prerequisites

```sh
export PATH="$HOME/.cargo/bin:$PATH"
rustup target add aarch64-apple-ios
rustup target add aarch64-apple-ios-sim
```

## Generate the Xcode project

```sh
cd crates/mozui-native/ios/TestHost
xcodegen generate
open MozuiIOSHost.xcodeproj
```

## Verify the simulator build

```sh
cd crates/mozui-native/ios/TestHost
xcodebuild \
  -project MozuiIOSHost.xcodeproj \
  -scheme MozuiIOSHost \
  -sdk iphonesimulator \
  -destination 'generic/platform=iOS Simulator' \
  -derivedDataPath /tmp/MozuiIOSHost \
  CODE_SIGNING_ALLOWED=NO \
  build
```

## Load it on a phone

1. Open `MozuiIOSHost.xcodeproj` in Xcode.
2. Set your Team in Signing & Capabilities for `MozuiIOSHost`.
3. Connect your iPhone and select it as the run destination.
4. Build and run. The prebuild script will compile `mozui-ios-demo` for `aarch64-apple-ios`.

If the device build fails before Xcode compiles Swift, install the Rust device target:

```sh
rustup target add aarch64-apple-ios
```

## What the host does

- builds `mozui-ios-demo` as a Rust static library during the Xcode build
- creates a `UIView` backed by `CAMetalLayer`
- attaches that view into the `mozui` iOS platform bridge
- pushes bounds, safe-area, scale-factor, appearance, and foreground/background events into Rust

## Current status

- `mozui-ios-demo` checks successfully for `aarch64-apple-ios-sim`
- `MozuiIOSHost` builds successfully for the iOS simulator
- physical-device build is expected once `aarch64-apple-ios` is installed and Xcode signing is configured
- the host now uses `UIScene` lifecycle instead of legacy app-window lifecycle

## Runtime note

If a debug run under Xcode still crashes with a `CaptureMTLDevice` selector error during Metal adapter setup, check the run scheme diagnostics and make sure:

- `GPU Frame Capture` is disabled
- GPU / Metal validation is disabled for the run

The generated XcodeGen scheme now requests those safer defaults, but an existing local scheme or Xcode override can still differ.
