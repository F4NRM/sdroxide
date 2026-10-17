# Undocked mode — START HERE (the next project)

**This is the fresh-session starting point (2026-10-09).** Read it top to bottom.
`main` is green, everything in the queue is merged, and the next piece of work
is the one below.

A new layout where chosen modules live in their **own OS windows** — like GIMP's
and SDRuno's detachable panels — so the main window can give its space to the
controls and decoders while the waterfall sits on a second monitor. Today every
`LayoutMode` (Desktop / Tablet / Small / Phone / Auto) stacks every module into
the **one** window; operators have asked repeatedly for the waterfall detached,
and the fork already has the mechanism (the solar3d viewport).

## TODO (do these in order)

- [x] **1. State.** `DetachedWindow` + `UiSettings::{panadapter_detached,
      panadapter_window}` — landed (`21c204fa`). No wire bump (local config).
- [x] **2. Extract `draw_panadapter`.** Both panadapter blocks — the digital
      path and the CW/analog path — lifted into one inherent
      `SdroxideApp::draw_panadapter`; the main-window draw is byte-identical
      (`747bfb82`). `cw_panadapter` and the render tests stayed green.
- [x] **3. The toggle, named "Undocked".** A **Settings → UI → Panadapter
      window** picker — *Docked in this window* / *Undocked — its own window* —
      with the Wayland caveat in its hover. Deliberately a settings choice
      rather than a strip chip: it is a workspace arrangement kept for good, not
      a mid-QSO control. Works in **every mode** (a voice mode leaves the main
      window's column to whatever the mode has; a digital mode leaves its
      operating panel). `close_requested` docks it again.
- [x] **4. The detached window.** `show_viewport_immediate` (native only) with
      app-id `sdroxide-panadapter` and a stable title. Immediate, not deferred:
      the draw borrows `&mut self`. One window for the **station**, owned by
      the focused radio (`UiSettings` is station-wide, so a per-radio id would
      remap the window on every switch).
- [x] **5. The main window reclaims the space.** `panadapter_split` returns a
      zero waterfall height when detached; pure test on the arithmetic plus the
      both-edges render test run detached too.
- [x] **6. Persist geometry** on the rebuild frame, and once a drag settles
      (not a write a frame).
- [x] **7. The overlay.** A big, display-only dial readout over the detached
      panadapter; the tuning strip and S-meter stay in the main window.
- [x] **8. Gate + install.** Workspace check silent, wasm check unchanged (273
      warnings), `sdroxide-ui` lib 757 passed, `cw_panadapter` green. Built and
      installed as `2.0.2_brown`; a live run on niri opened the window with
      app-id `sdroxide-panadapter` and persisted its geometry.

**Status (2026-10-09): the first slice, the second module, the shared helper
and the shell-owned window manager are built, committed and installed.** The
control lives in **Settings → UI** (one row per module, named **Undocked**), and
works in every mode.

**The module registry and the shared helper exist** (the step the handover called
"generalise to any detachable module"):
- `sdroxide_types::DetachableModule` (`Panadapter`, `Panel`) with
  `DetachedState { detached, window }`, held as an **array** in
  `UiSettings::detached` indexed by `module.index()` — an array and not a map
  so `UiSettings` stays `Copy`, which the whole UI leans on. A new module is a
  variant plus a UI spec. The pre-array `panadapter_detached` /
  `panadapter_window` keys migrate at load (`UiSettings::migrate_detached`).
- `frame.rs`: `DetachedWindowSpec` + `detached_spec(module)` (ids, app-id,
  title, size), one free `detached_viewport(ctx, spec, seed, draw)` that owns
  the whole multi-window plumbing, `panadapter_inputs()` so the panadapter draw
  is self-contained, `column_split(...)` for the arithmetic, and
  `dispatch_commands()` so a window's clicks are applied like the main
  window's.
- **Panel `app-id`: `sdroxide-panel`.** Panadapter: `sdroxide-panadapter`.

**The shell owns the windows now** (the handover's "shell-owned window manager"):
each frame `MultiApp` asks the focused radio's app which windows it wants
(`SdroxideApp::detached_wanted`) and emits those, once, whatever pane or tab is
on screen — so a radio or mode change can no longer tear a window down and have
a tiling compositor re-place it. `show_detached_module` is the app's half (how
to draw itself); the shell's is *whether the window exists*.

**The vacated centre is filled, and a mode with no panel opens no window**
(commit `24b6a3ce`, from the operator's screenshots). When both modules are
undocked the main window centre was a black hole; now `center_has_content()`
detects it and `band_menu_fill()` draws the band/mode selector there (the dock's
body is shared as `band_menu_body`). And a voice mode no longer opens a
placeholder panel window — `panel_window_wanted` requires a panel, and a
one-per-mode dismissible notice names the mode instead ("No operating panel in
AM …").

**The app-id and the window rules are documented** (`7c3b9ad5`): `app_id()` is a
method on `DetachableModule` (one source, used by the window spec and the
Settings hover), and the manual has an **Undocked** section under Settings → UI
with the app-id table and a **niri `window-rule`** example to float and pin the
windows to monitors (the app cannot place a window on Wayland; that is the one
thing the operator does, and it is documented rather than assumed).

**The panadapter window is self-sufficient** (`f7e4674a`): the big frequency
readout now has the **mode and signal level** under it (read-only, sharing the
main window's meter), so the second monitor needs no glance at the first. And a
**Dock all windows** chip in Settings → UI brings every window back at once.

**The console is SDRuno's RX control** (`0b3a78cd`): the undocked Controls window
is forced to the **desktop** strip (it opens wide and short, and the compact
strip was the first thing to look wrong), carries the **band/mode selector**
beneath it, and the main window's emptied centre shows a plain face instead of
drawing the selector a second time. The three dozen digital chips are one
**DIGITAL ▾** dropdown on both menu tabs, SDRuno's DIGITAL button.

**The control surface is a module** (`783ee680`): `DetachableModule::Controls`
moves the whole top strip — frequency, S-meter, receiver and transmitter
controls — into its own window (`sdroxide-controls`). Undocked, the main window
draws no top strip and becomes the spectrum and the decoders, while the strip
lives in the window: the classic SDRuno split of MAIN SP from RX control. Done by
moving the strip whole (not cutting its measured rows apart), publishing the
window's own tier while it draws. The three big modules the work was aimed at —
panadapter, panel, controls — are all detachable now.

**The destination, so the next slice aims at it:** a user-arrangeable workspace
in the SDRuno mould. The three docked modules **and the tool windows** are
detachable now:

- Adding a docked **module**: a `DetachableModule` variant, its `detached_spec`,
  a `show_detached_module` arm, and a `*_window_wanted` predicate where it is not
  always present — the registry, the shell and the Settings rows do the rest.
- Adding a **tool window**: extract its body to `fn <tool>_body(&mut self, ui, …)`
  and call `self.tool_window(ctx, "<id>", "<Title>", [w, h], open, |me, ui| …)`
  from its `*_window` method, storing the return back in its `show_*` flag. The
  DETACH/DOCK chip, the OS window, the geometry and the close-vs-dock handling
  are all in `tool_window`.
  **Done (13):** scanner, DRM, morse, signal id, contest, bands, known stations,
  RDS, HD Radio, Enigma, ISM, mail, satellite.
  **Left (bodies inline in their `*_window`):** schedule, logbook, spots,
  awards, grid tracker, public SDRs, recordings, SWL log, memories, voice keyer,
  and the per-mode setup windows (VDL2, ADS-B, AIS, FSK/FSQ, SSTV, WEFAX) — each
  just needs its closure body cut into a `_body` method. Nothing else is owed.

Everything below is the reference for those steps.

## What already exists (the groundwork — this is why it is not from scratch)

- **Multi-viewport is proven on this codebase.** The solar-system view is a real
  separate OS window, built with `ctx.show_viewport_deferred(vid, builder, …)`
  and an `egui::ViewportBuilder` — `crates/sdroxide-ui/src/solar3d/mod.rs`
  (`viewport_id`, around `:346`–`:366`). Any widget can be rendered into such a
  viewport; that is the whole of the "own window" mechanism.
- **Geometry persistence has a pattern.** `UiSettings::solar3d_window:
  Option<Solar3dWindow>` (`crates/sdroxide-types/src/ui.rs`) carries size and
  position, seeded into the builder only on the **rebuild frame**
  (`cumulative_pass_nr_for(vid) == 0`) so a live resize is never fought.
- **Per-radio window identity exists.** `layout::radio_salt(ctx)`
  (`crates/sdroxide-ui/src/layout.rs:275`) salts the viewport id per radio;
  `solar3d::viewport_id(salt)` builds it.
- **The one-window/one-owner rule exists.** The 3D window's `keep_alive` and
  owner cap (`solar3d/mod.rs:417`, and the AGENTS note "The 3D window in the
  multi-radio shell") are exactly the problem a detached panadapter has: a
  second window owned by a per-radio app, in a shell that draws only the visible
  tab. Read that note before touching any of it.
- **The layout enum is where a mode slots in.** `LayoutMode`
  (`sdroxide-types/src/ui.rs:244`) is `Desktop / Tablet / Phone / Small / Auto`;
  `layout::tier_for` (`:80`) maps it and the viewport size to a `Tier`. A new
  mode is appended **before `Auto`**, which must stay last because it carries
  `#[serde(other)]`.

## The known caveat, stated up front: Wayland, and **Niri**

A Wayland client **cannot set its own window position**. `viewport().inner_rect`
and `outer_rect` are `None` there (the solar3d notes record this) — only the
size comes back. So on Wayland "put it on monitor 2" is **best-effort**: size is
exact, placement is the compositor's. X11, Windows and macOS return real
geometry and honour a position. Say this in the UI copy rather than pretending.

**Niri is the operator's WM, and it is already a named target** in the "3D
window in the multi-radio shell" note (a remapped window is a *new* window to a
tiling compositor like niri/sway, so `keep_alive` is what holds its place).
Undocked mode meets Niri the same way, and the path is clearer than it looks:

- A detached viewport is a **new toplevel**, and Niri **tiles** it like anything
  else. To make it **float** — the operator's wish — is a Niri **window rule**,
  not an app call:
  ```kdl
  window-rule {
      match app-id="sdroxide-panadapter"
      open-floating true
      // and put it where the operator wants it:
      // open-on-output "HDMI-A-1"
      default-column-width { fixed 1200; }
      default-window-height { fixed 720; }
  }
  ```
- That is why the detached window must set **`with_app_id`** (and a title): it
  is the only handle a compositor rule has. Niri can also match on **title**, so
  even the 3D window (title only) is routable today — but a stable app-id is the
  better key.
- **`with_position` is ignored on Wayland/Niri.** Placement is `open-on-output`
  / `open-on-workspace` in the operator's rule, which is *better* for the
  "second monitor" case than anything the app could do.
- **`with_window_level`** (always-on-top) is compositor-dependent; do not rely
  on it. Niri floats and layers via its rules.
- **Emit the window every frame.** Niri destroys a window that stops being
  emitted and recompositor-places it on return — the `keep_alive` lesson, and
  the reason a detached panadapter (like solar3d) must stay alive while its tab
  is hidden.

Net: the app's job is to **expose the identity and the size**; *floating and
placing it on a chosen monitor is the operator's one-line rule*. That is the
honest division, and what the manual should say.

## Scope

**In (the first slice, below):** a detach toggle, the **panadapter** as one
detached window, its geometry persisted, and the main window reclaiming the
space.

**Out (later):** a general docking framework (any module, arbitrary splits and
tabbing); guaranteed multi-monitor placement under Wayland; floating in tiling
WMs; carrying the detached geometry on the wire (decide when it matters).

## First slice — the detached PANADAPTER

The single most-requested case, and the one that proves the pattern. Do not
build a framework first.

1. **State.** ✅ **Done (2026-10-08).** `DetachedWindow { size: [f32; 2], pos:
   Option<[f32; 2]> }` and `UiSettings::{panadapter_detached, panadapter_window}`
   are in `crates/sdroxide-types/src/ui.rs`, appended last, mirroring
   `Solar3dWindow`. **No `PROTO_VERSION` bump**: `UiSettings` is local
   `config.toml` and is *never* on the wire (it is `ClientScreen` that is — see
   its own doc). A bump would only be owed if the flag were carried in
   `ClientScreen` instead, which is the "does the detached geometry travel?"
   decision left to later. Machine-local is the default and what landed.
2. **The toggle.** A `DETACH` action in the panadapter's DISP row, with hover
   text saying where the window goes and the Wayland caveat. Turning it off (or
   closing the window — handle `ViewportEvent::Close`) re-attaches.
3. **The main window reclaims the space.** The digital split already computes
   `(waterfall_h, panel_h)` summing to the area (`frame.rs`, the `digi_split` /
   panadapter region). Detached ⇒ `waterfall_h = 0` and the operating panel
   takes the whole column. This is the part worth a **pure test**.
4. **The detached window — use `show_viewport_immediate`, not `_deferred`.**
   `spectrum_view::show_ext` borrows `&mut self`'s fields (view, state, peaks,
   smooth, trace cache, spec3d), and solar3d's `show_viewport_deferred` wants an
   `Fn + Send + Sync + 'static` closure — which **cannot** borrow `self`, so it
   would force the panadapter onto a published snapshot. Instead
   `Context::show_viewport_immediate(id, builder, impl FnMut(&mut Ui,
   ViewportClass))` (egui 0.36, `context.rs:4116`) is called in the **same
   frame** and may borrow `self`, so the existing draw moves over nearly
   unchanged. Build it with `ViewportBuilder::default().with_app_id(
   "sdroxide-panadapter").with_title(…)`, and `with_inner_size` /
   `with_position` from the persisted geometry. **The `with_app_id` is not
   decoration**: it is what a Wayland window rule matches on to float and place
   the window (see "Niri" below). solar3d sets only a title today; a detached
   window should carry its own id.
5. **Persist geometry.** Capture `inner_rect` / `outer_rect` on the rebuild
   frame only (the solar3d pattern), so a drag is not undone each frame.
6. **Native only.** `#[cfg(not(target_arch = "wasm32"))]` throughout the detach
   path; the browser keeps the panadapter in-window.

**What the detached window contains (decided 2026-10-08).** Spectrum + waterfall
+ a **big centred frequency** readout over it (the `centred_waterfall_note`
pattern), and optionally a compact S-level. The **tuning strip and the real
S-meter stay in the main window** — one owner for controls, so a value can never
disagree with itself across two windows, and the panadapter stays a *view*
(click/drag tuning on it still works, it is the same widget).

### Step 2 in practice — the extraction

`frame.rs`'s panadapter block is a deeply nested closure (`ui.allocate_ui(
vec2(width, wf_h), |ui| { … spectrum_view::show_ext(…) })`, roughly
`frame.rs:765`–`:904`) capturing many locals (`mode`, `live`, `wf_tuning`,
`atsmini`, `audio_hz`, `markers`, `ft8_spots`, `ft8_alpha`, `net_spots`,
`net_alpha`, `clicked_spot`, `ism_labels`, `mem_marks`, `pan`, `show_panel`,
`cmds`, `now`, `frame`) plus `self` fields. The work is to lift its body into
`fn draw_panadapter(&mut self, ui, <those by-ref>)`, then call it from:

- the main window, when **not** detached (`if show_wf && !detached`);
- the immediate viewport, when detached — `ctx.show_viewport_immediate(vid,
  builder, |ui, _| ui.allocate_ui(viewport_size, |ui|
  self.draw_panadapter(…)))`.

**Do that extraction as its own commit and land the main-window draw
byte-identical first**, so a mistake in the move is visible before any of the
viewport is wired. Only then add the toggle, the geometry persistence and
`ViewportEvent::Close`.

**The oracle.** The detached window itself cannot be asserted headlessly, so the
test is the **main-window layout**: with the panadapter detached, the operating
panel takes the full height and still stays inside its column — the same
both-edges check as `the_operating_panel_stays_inside_its_column`
(`crates/sdroxide-ui/src/app/mod.rs`). Plus a pure unit test of the split
arithmetic (`waterfall_h == 0`).

## Workflow and house rules (do not skip)

- **Gate before any tag:** `cargo check --workspace --all-targets` **silent**,
  then `cargo test --release --workspace`.
- **The wasm check is mandatory** whenever a wasm-reachable file changes:
  `cargo check --release --target wasm32-unknown-unknown -p sdroxide-ui`. The
  detach must not pull a native-only path into the browser build — the DAB
  radio-tab move (`5bef39e7`) broke exactly this and the check caught it.
- **Wire:** only **`ClientScreen`** rides the wire, whole — `UiSettings` does
  **not** (it is local `config.toml`; putting it on the wire is the trap its own
  doc warns about). So a field added to `UiSettings` needs no bump; a
  `ClientScreen` field or a new `LayoutMode` variant does — a `PROTO_VERSION`
  bump plus a register line in `crates/sdroxide-proto/src/lib.rs`. `LayoutMode`
  is appended **before `Auto`**.
- **A new `LayoutMode` variant** is also a `ClientScreen` change and is
  validated by the "catch-all last" rule — read `Auto`'s own doc first.
- **Multi-radio is the risk area.** Reuse the owner-cap/`keep_alive` model from
  solar3d; do not leave N radios each emitting a window.

## Later, when the slice works

- **Generalise to any detachable module** (decoders, band/mode dock) on the same
  `DetachedWindow` type, rather than a boolean per module.
- **The shell-owned window manager.** The AGENTS note already names the real
  fix: "a shell-owned window manager with stable viewport ids and an explicit
  owner, emitted every frame regardless of focus". A general undocked mode is
  the natural home for it.
- **Floating in tiling WMs** (the operator runs **Niri**): a tiling compositor
  tiles a new toplevel; making it float, and pinning it to a monitor, is a
  **Niri `window-rule`** the operator sets (see the Niri section above) — not
  something a Wayland client can assert. The app's contribution is the
  **app-id** the rule matches on. Worth a short manual section; not an app
  feature.
  **Update (2026-10-17): the X11 half is now an app feature.** The **SDRuno
  workstation** (`SDRUNO-UI-HANDOVER.md`) sets
  `ViewportBuilder::with_window_type(X11WindowType::Utility)` on every window it
  opens, which is `_NET_WM_WINDOW_TYPE_UTILITY` and is honoured by i3, sway,
  bspwm, awesome and the rest — a program *can* ask to float, on X11. Wayland
  still has no call for it, so niri remains the operator's rule.
- **Whether the detached geometry travels** on `ClientScreen` (a remote client
  that wants its own arrangement) — a decision, not a default.

## Bench checks before committing to it

1. **Does a second viewport render the wgpu waterfall cleanly, and at what
   cost?** The panadapter is the GPU-heavy widget; the 3D window is the only
   precedent and it is a different render path.
2. **Multi-radio:** who owns the detached panadapter, and what a tab switch
   does (the solar3d owner-cap problem, exactly).
3. **Wayland:** confirm on niri/sway that a detached window can at least be
   resized and moved by the operator, and that the app reads a sane size back.
