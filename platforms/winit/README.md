# AccessKit winit adapter

This is the winit adapter for [AccessKit](https://accesskit.dev/). It exposes an AccessKit accessibility tree through the platform-native accessibility API on any platform supported by AccessKit. On platforms not supported by AccessKit, this adapter does nothing, but still compiles.

## Compatibility with async runtimes

The following only applies on Linux/Unix:

While this crate's API is purely blocking, it internally spawns asynchronous tasks on an executor.

- If you use tokio, make sure to enable the `tokio` feature of this crate.
- If you use another async runtime or if you don't use one at all, the default feature will suit your needs.

## Android activity compatibility

The Android implementation of this adapter currently only works with [GameActivity](https://developer.android.com/games/agdk/game-activity), which is one of the two activity implementations that winit currently supports.

## Downstream compatibility prototype

This fork branch keeps AccessKit 0.24-compatible consumers working while testing
egui 0.36 with winit 0.31. It is not an upstream PR or a supported prerelease.
Only the default rwh_06 path is validated; the old optional rwh_05 path is not
supported by winit 0.31. Linux application tests and a Windows cross-check passed
in the downstream integration. Apple and Android runtime validation is pending.
The current-main callback proposal is on prototype/winit-031-callback instead.
