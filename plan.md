Long-term Plan: Linux Tray Icon Support and Example Builds

Goal
- Ensure cargo check -p gpui --example tray passes successfully on Linux (Wayland and X11).

Current Status
- gpui builds for the library. Wayland/X11 tray implementation wired to StatusNotifierItem with DBusMenu dispatch.
- Fixed: GPU context initialization on Wayland/X11, cursor style delegation recursion, Pixmap i32 sizing.

Risks/Constraints
- GUI examples may require native system libraries to link/run in CI. The benchmark for completion is cargo check for the tray example, which should not require system libs to link executables.

Next Steps
1) Keep Linux tray code compiling across both Wayland and X11 targets.
2) Keep StatusNotifierItem menu wiring stable and typed; avoid lifetime/closure issues.
3) Maintain headless mode compatibility so repository builds in environments without DISPLAY/WAYLAND_DISPLAY.
4) If CI requires it, gate optional features that pull in native deps for examples during cargo check.
5) Continuously re-run cargo check -p gpui --example tray after changes.

Work Log
- 2025-12-24: Adjusted Linux platform code
  - Converted Pixmap width/height to i32.
  - Replaced notify_err calls with expect for BladeContext::new().
  - Fixed event source registration return handling (.log_err() no longer followed by .ok()).
  - Delegated set_cursor_style to LinuxClient implementation to avoid recursion.
  - Added KEYRING_LABEL constant and cleaned imports in linux platform module.

Completion Benchmark
- Command: cargo check -p gpui --example tray
- The task is complete when the above command succeeds.
