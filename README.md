# twisty-cube
CLI: `cube-cli`. Desktop app: FaceTurn Cube Solver.

Two-phase and optimal HTM solver, plus a desktop shell that shows the cube.

The solver crate is already a library (`cube-cli`). `cargo run --bin cube-cli` is the same CLI as before. `apps/cube-tauri` is a Tauri 2 window: orbit the cube, turn faces, run two-phase, and play the solution back.

## Tables

Two-phase search reads `manifest.bin` and the ten table files next to it. It does not build them on a solve, and it does not load `phase1_prun.bin`.

```bash
cargo run --release --bin cube-cli -- gen-tables
```

`gen-tables` writes about **7.2 MB** into `./tables` (gitignored).

The desktop shell looks for `manifest.bin` in this order:

1. `$CUBE_TABLES` (dev override)
2. `tables/` inside the app bundle (`resource_dir()/tables`)
3. Application Support `com.dougcoburn.faceturn/tables` (macOS: `~/Library/Application Support/…`; elsewhere `~/.local/share/…`)
4. `./tables` from the current working directory
5. `tables/` beside this crate (`CARGO_MANIFEST_DIR`)
6. `tables/` beside the executable

The CLI (`cube-cli` and `cube-preview`) still uses only `$CUBE_TABLES`, `./tables`, the crate directory, and the executable directory. It does not generate into Application Support.

Solve never writes into the `.app`. If no directory above has `manifest.bin`, the desktop app generates into Application Support on a background thread and emits `tables-progress` / `tables-done`. If that generate fails, Solve names the paths it tried and says to reinstall the app or run `cube-cli gen-tables`.

`cargo tauri build` copies `../../../tables/` into the bundle as `tables/`. That directory is gitignored. A Mac App Store or release build has to run `gen-tables` first; `src-tauri/build.rs` prints a warning when `manifest.bin` is absent. The binaries are not committed.

## Desktop app

macOS is the target. Install the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) (Xcode command line tools), then:

```bash
cd apps/cube-tauri
npm install
cargo tauri dev
```

The window talks to the solver with `get_solved`, `apply_moves`, `validate_facelets`, `scramble`, and `solve_twophase`. The solve command runs on a blocking thread so the webview stays responsive. Optimal search stays on the CLI (`--mode optimal`) and is not wired in the window.

## What to click

1. Reset shows six solid faces. Drag to orbit. The cube does not spin on its own.
2. U (button or the U key) turns the white face clockwise. Shift+U is U'. Press 2, then a face, for a half turn.
3. Shuffle 25, Solve, Play. The cube should end solved. The status line shows `length`, `optimal false`, and elapsed milliseconds.
4. Paste a move string and Apply moves, then Solve and Play.
5. A 53-character facelet string, or any illegal coloring, shows an error and leaves the window up.

Playback keeps the facelets from when you pressed Solve. Step back animates the inverse. The slider jumps to that prefix without replaying the earlier moves.

## Tests

```bash
cargo test
```

`solve_twophase` on a solved cube is empty, and applying a two-phase solution to a 5-move scramble returns to solved. The exhaustive superflip proof stays `#[ignore]`.

## Mac App Store

Bundle ID: `com.dougcoburn.faceturn`. Product name: FaceTurn Cube Solver. Category: Education (`public.app-category.education`). `LSMinimumSystemVersion` is 12.0, which is the floor used here for Tauri 2 on Apple Silicon. `ITSAppUsesNonExemptEncryption` is false; the solver does not use non-exempt encryption. Copyright is `Copyright © 2026 Doug Coburn`.

This repo does not sign or upload. Certificates, the Team ID, and the provisioning profile stay on your machine.

### What you do in Apple Developer

1. Enroll in the Apple Developer Program.
2. Register an App ID for `com.dougcoburn.faceturn` with App Sandbox.
3. Create a Mac App Store Connect provisioning profile for that App ID.
4. Create two certificates: **3rd Party Mac Developer Application** (signs the `.app`) and **3rd Party Mac Developer Installer** (signs the `.pkg`).
5. In App Store Connect, create the API key used by `altool`. Note the key id and the issuer id.

### What stays out of git

Put the profile here (the path is gitignored):

```text
apps/cube-tauri/src-tauri/embedded.provisionprofile
```

`apps/cube-tauri/src-tauri/tauri.appstore.conf.json` points the bundle at that file. Do not commit `.p8`, `.p12`, `.cer`, `.provisionprofile`, `.mobileprovision`, or `AuthKey_*.p8`. Those patterns are in `.gitignore`.

`Entitlements.plist` enables the App Sandbox only. There is no network, camera, or USB entitlement. Replace both `YOUR_TEAM_ID` strings with the Team ID from the App ID prefix (Certificates, Identifiers & Profiles). `codesign` does not expand `$(AppIdentifierPrefix)`. If you treat the Team ID as private, do not commit the replaced file.

Signing identities are environment variables, not config:

```bash
export APPLE_SIGNING_IDENTITY="3rd Party Mac Developer Application: Your Name (YOUR_TEAM_ID)"
export APPLE_API_KEY_ID="your-key-id"
export APPLE_API_ISSUER="your-issuer-id"
```

The `.p8` must be named `AuthKey_$APPLE_API_KEY_ID.p8` and live in one of the directories `altool` searches (`~/private_keys`, `~/.private_keys`, `~/.appstoreconnect/private_keys`).

### Tables in the bundle

From the repo root, before `tauri build`:

```bash
cargo run --release --bin cube-cli -- gen-tables
```

That writes about 7.2 MB into `./tables`. The bundle maps that directory to `Contents/Resources/tables`. At runtime the app reads it from `resource_dir()/tables` and never writes there. First launch with no bundled tables generates into `~/Library/Application Support/com.dougcoburn.faceturn/tables` instead.

### Build the .app and the .pkg

Commands follow the [Tauri App Store page](https://v2.tauri.app/distribute/app-store/). Run them from `apps/cube-tauri` after `npm install`. Without a certificate, `tauri build` still produces an unsigned `.app` you can open locally; signing fails only when you ask for a signed store package.

```bash
npm run tauri build -- --bundles app
```

The unsigned (or Developer ID) app is under `src-tauri/target/release/bundle/macos/FaceTurn Cube Solver.app`.

Mac App Store bundle, after the profile and `YOUR_TEAM_ID` are in place:

```bash
npm run tauri build -- --no-bundle
npm run tauri -- bundle --bundles app --target universal-apple-darwin --config src-tauri/tauri.appstore.conf.json
```

Wrap and upload (do not run the upload from CI in this repo; it needs your API key):

```bash
xcrun productbuild --sign "3rd Party Mac Developer Installer: Your Name (YOUR_TEAM_ID)" \
  --component "src-tauri/target/universal-apple-darwin/release/bundle/macos/FaceTurn Cube Solver.app" \
  /Applications "FaceTurn Cube Solver.pkg"

xcrun altool --upload-app --type macos --file "FaceTurn Cube Solver.pkg" \
  --apiKey "$APPLE_API_KEY_ID" --apiIssuer "$APPLE_API_ISSUER"
```

After a sandboxed build, confirm the window opens, Shuffle → Solve → Play still finishes solved, and Console has no sandbox denials for that path. Tables should come from the bundle or from Application Support, not from inside the `.app` as a write.

## License
Licensed under MIT. See LICENSE.

## Credits
Two-phase search follows Kociemba. Optimal search follows Reid (1997).
Not affiliated with Spin Master or the Rubik’s Brand.
