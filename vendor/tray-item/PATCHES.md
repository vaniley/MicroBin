# Local tray-item 0.10.0 patch

Source: https://crates.io/crates/tray-item/0.10.0
Upstream: https://github.com/olback/tray-item-rs
License: MIT; the upstream LICENSE is retained.

The source modules and original package manifest come from the published 0.10.0 crate. Unrelated package examples and tooling are omitted. The workspace root controls build profiles. Formatting follows the workspace formatter. Six pre-existing Windows Clippy style warnings receive mechanical fixes so strict workspace lint checks remain clean.

## Windows recovery patch

- Keep the full desired NOTIFYICONDATAW in shared window state.
- Update that state whenever the icon or tooltip changes.
- Handle TaskbarCreated by re-adding the saved icon, tooltip, identity, and callback message.
- Do not load a generic icon or quit the message loop on a recovery failure.
- Register TaskbarCreated once per window, with error handling, instead of using a global mutable message ID.
- Copy saved state before calling Shell_NotifyIconW. Do not hold its mutex during Shell calls.
- Preserve existing handle ownership: RawIcon belongs to the caller, which must keep it alive until the tray is destroyed.

Regression tests in the Windows structs module cover repeated recovery, custom and fallback icon handles, Unicode and updated tooltips, callback identity, unrelated messages, and a zero message ID. Run on Windows:

```powershell
cargo test --locked -p tray-item --lib
```

The root package remains the default workspace member so Linux configuration checks do not build a desktop tray backend.
Remove this patch only after an upstream replacement passes the same recovery tests.
