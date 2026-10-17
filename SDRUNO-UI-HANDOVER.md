# SDRuno-style UI — the window map and the todo (handover)

**Fresh starting point (2026-10-09), read this before the SDRuno-shaped UI work.**
The operator wants this program's window layout to read like **SDRuno**: a set of
windows, each its own module, that the operator arranges. The undocked mode
(`UNDOCKED-HANDOVER.md`) is the machinery; this file is the **shape** being aimed
at and what is left. It exists because the operator expects to **switch models
when the work reaches MAIN SP** — so note the split below.

**Where the work stands: the band keypad (item 1) is built, tested and gated, and
the next item is MAIN SP — the model-switch seam. Nothing has been started past
it, deliberately.**

## SDRuno's windows, from the manual (v1.4, ICAS)

Source: `https://icas.to/sdrplay/SDRuno/sdruno-usermanual-jpn-1a-140.htm`
(Japanese; the operator translates it). The window set:

1. **MAIN** — controls all of SDRuno. Its buttons open the rest: **SP1**, **SP2**,
   **RX**, **SCANNER**, **REC PANEL**, **MEM PAN**, plus SETT., PLUGINS, SAVE WS.
2. **RX Control** — frequency readout, mode, filter, NB/notch, AGC, squelch,
   volume, the **band keypad**, and SETT.
3. **MAIN SP (SP1)** — the main spectrum + waterfall.
4. **AUX SP (SP2)** — a second spectrum + waterfall.
5. **PLUGINS** — the decode plugins.
6. **RECORDER** — record/playback.
7. **MEM. PANEL** — memories.
8. **SCANNER**.

## Our mapping

| SDRuno | ours | state |
|---|---|---|
| MAIN | the `MultiApp` shell + the radio tab strip | exists (no separate window) |
| RX Control | the **Controls** module (`sdroxide-controls`) | built: strip, band keypad, band/mode list |
| MAIN SP (SP1) | the **Panadapter** module (`sdroxide-panadapter`) | built, with its toolbar |
| AUX SP (SP2) | `DetachableModule::AuxPanadapter` | built |
| RX-control band panel | `DetachableModule::BandMenu` (`sdroxide-bandmenu`) | built (2026-10-10): the band/mode selector in its own window, five columns |
| SCANNER / RECORDER / MEM. PANEL / … | the **tool windows** with the ⇱ WINDOW chip | mechanism done; ~13 tools converted, the rest need their bodies extracted |
| PLUGINS | the decoders are modes, not plugins | n/a |

## What is done (the machinery)

- **Module registry + shell-owned window manager** (`UNDOCKED-HANDOVER.md`): four
  docked modules — Panadapter, Panel, Controls — each with a `detached_spec`,
  emitted by the shell; geometry persisted; close-to-dock.
- **Tool windows**: a general `tool_window(ctx, id, title, size, open, body)`
  draws an egui window or its own OS window; a ⇱ WINDOW/DOCK chip toggles it.
  Converted: scanner, DRM, morse, signal id, contest, bands, known stations, RDS,
  HD Radio, Enigma, ISM, mail, satellite.
- **The Controls console** is SDRuno's RX control in shape: desktop strip (never
  the phone strip) + the band/mode selector beneath it; the digital modes are one
  **DIGITAL ▾** dropdown; the emptied main window shows a plain face, not the
  selector a second time.
- **The band keypad** (2026-10-09) — the console's left column: **Bands**/**MHz**
  above, ten keys in calculator order, **Clear**/**Enter** below, with the full
  band list beside it. The manual describes it under **Undocked**.

## TODO (in order)

1. ~~**The band keypad.**~~ **BUILT (2026-10-09)** — `top_bar::band_keypad` and
   the console's left column, with the reasoning below.
2. ~~**MAIN SP — switch model here.**~~ **BUILT (2026-10-09)** — the panadapter
   window now carries an SDRuno-style **toolbar**: the frequency readout (with the
   mode and S-meter) in a bar across the top instead of floating over the middle
   of the waterfall, a **DISP** chip opening the same layer menu the main window's
   SPEC chip does, and a **DOCK** chip that brings the window home. See the note
   below. The operator's "switch models here" was honoured by handing this slice
   back as a self-contained commit rather than a sprawling one.
3. ~~**AUX SP (SP2)** — a second panadapter window.~~ **BUILT (2026-10-09)** —
   `DetachableModule::AuxPanadapter`, its own app-id, its own window, its own
   SP1 toolbar. See the note below.
4. ~~**The MAIN window's buttons.**~~ **BUILT (2026-10-09)** — the **🪟 WINDOWS**
   chip on the radio-tab strip: the four module windows by their SDRuno names,
   each a toggle, plus **DOCK ALL WINDOWS**. See the note below.
5. ~~**The rest of the tool windows.**~~ **BUILT (2026-10-10)** — every window on
   this list now goes through `self.tool_window`, so each has a **⇱** chip and a
   remembered geometry. **Item 5 is closed.** The `&mut` locals and the pre/post
   values were the whole difficulty and none of it turned out to be a design
   decision: a body takes them as `&mut` parameters and returns through them,
   exactly as the existing five already did.

   | window | id | size | carried back |
   |---|---|---|---|
   | SCHEDULE | `schedule` | 820×560 | `tune`, `log`, `fav_toggle` |
   | RECORDINGS | `recordings` | 720×480 | `new_job`, `save`, `cancel`, `edit`, `delete` |
   | SPOTS | `spots` | 580×480 | `clicked`, `open_setup` |
   | PUBLIC SDRS | `public-sdrs` | 980×520 | `refresh`, `picked`, `answer` |
   | VOICE KEYER | `voice-keyer` | 600×400 | — (commands emitted in the body) |
   | AIS setup | `ais-setup` | 420×420 | config copy/diff |
   | ADS-B setup | `adsb-setup` | 400×420 | config copy/diff |
   | VDL2 setup | `vdl2-setup` | 420×460 | config copy/diff |
   | FSQ Contacts | `fsq-contacts` | 320×360 | `changed`, `set_target` |
   | digimode setup | `digi-setup` | 420×460 | dynamic mode title |

   **A real bug fell out of it**: RECORDINGS' **edit** button was dead — the
   `edit` local was set and never read, so choosing a job and pressing EDIT did
   nothing at all. It now loads the selected job into the form. That is the
   "a control that can silently do nothing" rule finding a second instance in
   code this refactor touched.

   **Settings joined them (2026-10-10)** — id `settings`, app-id
   `sdroxide-tool-settings`. It needed two things the other ten did not:
   `tool_window_sized` grew a `min_size` and a `centre_first`, because the
   settings tabs are laid out at fixed widths (a narrow box is a column of
   clipped controls) and a 900x760 window in a corner sits *under* the 187 pt
   top bar. **Rendering it caught the second one**: the conversion dropped the
   old `default_pos` and the box opened at `x 15.4..916.6 y 15.4..776.6` in a
   1920x1080 window, entirely under the top bar. Pinned by
   `the_settings_box_opens_below_the_top_bar`, verified failing with the
   centring off (`the settings box opened at y 15, under the top bar`).
   Its window's `vscroll` is now an explicit `ScrollArea::vertical()`, built
   *before* `bars.restore(ui)` so the scrollbar keeps its accent.

   **WEFAX is the one deliberate omission.** Its only window is the per-chart
   **viewer** (`panels/wefax.rs`), opened from a selected chart and carrying that
   chart's own title, size and state — not a menu tool. A fixed tool id would
   make one window serve every chart, and a ⇱ chip on a viewer that exists only
   while a chart is selected buys nothing. Leave it docked.
6. ~~**Workspace save/recall** — SDRuno records up to ten named workspaces.~~
   **BUILT (2026-10-09)** — `sdroxide_types::Workspace`, a `workspaces.json` list,
   and a **Workspaces** section on Settings → UI. See the note below. **Ctrl+W is
   built too (2026-10-10)** — `Action::Workspaces` appended last, `PROTO_VERSION`
   198 → 199, schema stamp 2 → 3 so an existing `input.json` picks the key up.

## The band keypad as built (2026-10-09) — and where it deliberately stops

- **A widget of its own in the console**, not a part of `band_mode_menu`: the
  popup and the dock are narrow and the pad is not. It is drawn by
  `show_detached_module`'s `M::Controls` arm, beside the band list, and it is
  **console-only** (`#[cfg(not(target_arch = "wasm32"))]` throughout) — the
  browser has no console.
- **Digits, calculator order** (`KEYPAD_ROWS`): `7 8 9 / 4 5 6 / 1 2 3`, then `0`
  centred alone under them. **The bands rise with the digits** — `1` is 160 m and
  `9` is 11 m — so the pad reads up the way the dial does.
- **The ten bands, and what is not on the pad.** A calculator grid has ten keys.
  They are the harmonic HF allocations; **60 m** (a 15 kHz secondary allocation)
  and everything above 6 m are not on it. 11 m is, because it is this program's
  band — it took the `9` key, which is why 60 m rather than 11 m is the one left
  off.
- **The digit-to-band table is ours.** SDRuno's own assignment was not published
  in anything we could read, and a table invented and then described as theirs
  would be a claim nobody checked; what is documented here is the *ordering*
  rule, which is the part a muscle memory forms on.
- **MHz mode types a frequency in kilohertz**: `14074` is 14.074 MHz, six digits
  deep, `ENTER` sends it and hands the keys back to the bands, `CLEAR` empties it
  and stays in MHz. ENTER is refused, with the range named, when the radio
  publishes a receive range the typed frequency falls outside.
- **The band list stays beside the pad, and that was the decision to make.** It
  was **not** moved behind a **MORE BANDS ▾** dropdown: our list carries 28
  bands — CB, the VHF/UHF allocations, the microwave bands, the broadcast
  services — and hiding them behind a dropdown to put ten keys where they were
  would make them *harder* to reach than they are today. The **DIGITAL ▾**
  dropdown exists because three dozen chips overflowed a row, not because a
  keypad replaced them. The two stack below `keypad_side_by_side_w(ui)`, which is
  the list's own wrap threshold — the same constant `band_pad` reads, so the two
  cannot drift apart.
- **One press, one meaning**: `band_jump_target` is now the single place that
  decides what pressing a band does in the current mode, and both the list's
  chip and the keypad's key go through it.
- **Tests** (nine, `top_bar::tests`): one band per digit and one digit per band;
  the bands rise with the digits and the rows are a calculator's; a key is a band
  in one mode and a digit in the other; the entry is kilohertz and six digits;
  ENTER tunes and returns the pad to the bands, an empty ENTER tunes nothing;
  CLEAR empties without leaving the mode; the readout says which of the two it
  is; no key is narrower than its own label (measured against the style — the
  Terminal theme's brackets are 14 pt of key width apiece); and **a render test**
  that ten keys land in `3,3,3,1` rows with `0` centred and nothing painted past
  the column. The render test and the width test were both verified failing on
  wrong code (`KEYPAD_KEY_AIR = -40`, and `0` moved to the left column).

## MAIN SP as built (2026-10-09) — the toolbar

- **The bar.** `sp1_toolbar` (in `frame.rs`, console-free, native-only) draws a
  fixed-height (`SP1_TOOLBAR_H = 32 pt`) header across the top of the undocked
  panadapter window, and the spectrum is drawn **below** it. The frequency readout
  that used to float over the middle of the waterfall — `centred_detached_readout`,
  now deleted — sits in the bar with the mode and the S-meter beside it.
- **DISP** opens the **same** layer menu the main window's SPEC chip does
  (`layers_button` → `panadapter_controls`), so SP1's toggles are the program's
  own rather than a second copy. It needed one visibility change:
  `layers_button` is now `pub(in crate::app)`.
- **DOCK** sets `DetachableModule::Panadapter` back to docked and persists, so the
  operator does not have to hunt for the window's close box to send it home.
- **The readout stays display-only** — a click or drag on the spectrum still tunes
  and the tuning strip stays in the controls window — preserving the single-owner
  invariant for the frequency.
- **Test** `the_sp1_toolbar_is_a_header_and_not_a_share_of_the_picture`: the bar is
  tall enough for the 21 pt readout and under a tenth of the window SP1 opens at.
- **Not seen in a running window** (the detached viewport no-ops under the headless
  harness), and the bar reserves height the spectrum then draws into.

## AUX SP as built (2026-10-09) — the second spectrum, and what it is not

- **`DetachableModule::AuxPanadapter`**, appended (slot 3), app-id
  `sdroxide-panadapter-aux`, window title *aux panadapter*, opening size the same
  as SP1. A Settings → UI row appears for it by itself: that list walks
  `DetachableModule::ALL`, and the hover names its app-id for a window rule.
- **One draw, two windows.** `M::Panadapter | M::AuxPanadapter` is a single match
  arm, so the two cannot drift apart in anything but their id. `module_window_wanted`
  is the one predicate for both ("undocked, focused, and the layers are on"), so a
  second spectrum window never appears without a spectrum in it.
- **The toolbar takes the module it is in**, so AUX SP's **DOCK** sends *AUX SP*
  home rather than SP1 — the bug a shared toolbar would otherwise have.
- **It is a second view of the *same* receiver**, and the doc says so. Both
  windows render one station's shared spectrum frame and the same `ViewState`, so
  they show the same band and the same zoom. Pointing AUX SP at another radio
  needs the shell to own a window per (module, radio) rather than per module,
  which is a bigger change than this slice and is left for whoever wants it.
- **The registry is now proven to extend safely** — that was the real work here.
  Adding a module grows `UiSettings::detached`, and a derived array deserializer
  would have rejected every saved config over one extra slot; `Settings::load`
  **quarantines** a file it cannot parse and answers `Settings::default()`, so
  the cost would have been the operator's theme, fonts and layout as well as
  their window geometry. `detached_slots` reads the field at any length now, and
  the tests are in `crates/sdroxide-config/tests/detached_slots_survive_a_module_count.rs`
  (through the real loader) and `ui::tests::the_detached_list_loads_at_any_length`.

## The MAIN window's buttons as built (2026-10-09) — WINDOWS on the strip

- **On the radio-tab strip, not the top bar.** That was tried first and the strip
  layout tests refused it: the System box's width is priced from its two chip
  rows and the top row is the wider of the two (435 pt against 427), so an eighth
  chip there grew the box by ~85 pt and the desktop strip needed **three** rows
  instead of two. The radio-tab strip is also the better home on the handover's
  own reading — it *is* the MAIN window, and every other control on it is about
  the windows and the radio together.
- **The chip** (`MultiApp::windows_button`) is lit while any module is out of the
  main window, and opens one row per module named as an SDRuno operator looks for
  them — **SP1 — the spectrum**, **SP2 (AUX) — a second spectrum**, **Operating
  panel**, **RX control** — each with a **DOCKED** / **IN ITS OWN WINDOW**
  toggle, and **DOCK ALL WINDOWS** when any is out. The toggle closes the menu on
  **DOCK ALL** rather than leaving a list of rows that are all docked.
- **It acts on the focused radio**, and reads its settings rather than the app
  mutably: the strip is drawn from `&MultiApp` and a split pane must not undock
  the radio it is not looking at. So the change goes through
  `StripAction::{SetDetached, DockAll}` and lands in
  `SdroxideApp::{set_module_detached, dock_all_windows}` — the same two writers
  Settings → UI uses, so the two cannot drift in whether they persist.
- **The browser draws no chip.** It has one window and keeps every module in it,
  so there is nothing to arrange; the strip's calls are `#[cfg]`-gated to keep
  that honest rather than leaving a dead control.
- **Test** `the_windows_menu_offers_every_module_by_name`: one row per module, no
  two rows the same, and SP1/SP2 called by those names.
- The module names live in one function (`module_window_name`), so a module added
  to the registry gets a row whether or not anyone remembered.

## Tool windows converted so far (2026-10-09)

- **AWARDS**, **LOGBOOK**, **GRID TRACKER**, **Memories** and **SWL LOG** now go
  through `self.tool_window`, so each has the ⇱ WINDOW chip and can sit on a
  second monitor. Each is `fn x_body` + a caller that keeps only the tail (what
  the body asked for afterwards).
- **The borrow that makes it awkward, and the two answers.** A window whose body
  captured `&mut` locals cannot be moved into a closure the shell calls while it
  holds the app. The five done each took one of two routes: the locals became
  **parameters** of the body (`swl_log_body(..., mark_sent: &mut Option<u64>)`),
  or state already out of the app was **taken and put back**
  (`std::mem::take(&mut self.grid_tracker)` — the shell borrows the app for the
  window, so the tracker cannot be borrowed out of it at the same time).
- **Not done on purpose**, and the reason is in the TODO above: SPOTS and PUBLIC
  SDRS need their results carried back through the app, which is a decision about
  where a window's actions land, not a body extraction.

## Workspaces as built (2026-10-09) — save and recall the arrangement

- **`sdroxide_types::Workspace`**: a `name` and a `Vec<DetachedState>` (one slot
  per module at save time). `capture(name, &UiSettings)` snapshots;
  `apply(&mut UiSettings)` restores, leaving any module a shorter list does not
  mention docked. `WORKSPACE_MAX = 10`, SDRuno's ten.
- **A list of its own, not a `UiSettings` field.** `UiSettings` is `Copy` and the
  whole UI passes it by value; a `Vec` in it would take that away everywhere to
  add one screen's worth of arrangements. So workspaces live in `workspaces.json`
  via `sdroxide_config::{load,save}_workspaces` (through `load_json_list`, so one
  unreadable row costs only itself). `sdroxide-ui` reaches it through
  `persist`, because `sdroxide-config` is native-only.
- **The UI is on Settings → UI**, under the undocked rows it acts on: a name
  field + **SAVE CURRENT**, and one chip per saved workspace (**×** to forget,
  the name to apply). Applying sets `cfg.detached`, so the shell re-emits windows
  next frame with no restart. Saving over an existing name replaces it rather
  than making a duplicate.
- **The clones-out / commits-back plumbing** matches `ui_edit`: the settings
  window is drawn from `&self`, so the list and the name travel into `SettingsIo`
  and are committed (and `workspaces.json` written) when the dialog closes with a
  change.
- **Tests**: `ui::tests::a_workspace_round_trips_the_arrangement` (including a
  shorter list a newer build would carry) and
  `sdroxide-config/tests/workspaces_round_trip.rs` (through the real loader, with
  a bad row proving the list costs only itself).
- **Ctrl+W (2026-10-10)**: `Action::Workspaces`, appended last on the enum,
  `PROTO_VERSION` 198 → **199** (`VERSION_BYTE` 0x13 → 0x14), schema stamp
  2 → **3** with `(3, Action::Workspaces)` in `InputSettings::ADDED` — so an
  operator who already has an `input.json` **gets the key** rather than a dead
  shortcut, and one who rebound or deleted it keeps their choice.
  **It opens the Settings window on the UI page** rather than toggling: the
  workspaces are a section of that page, there is no workspaces window of its
  own, and a toggle would close the window an operator just used to save one.
  `UiSink` carries a `workspaces: &mut bool` for the same reason it carries
  `speech` — picking the page is `frame.rs`'s call. **Not tested at frame
  level**: the dispatch arm is
  (`ui::input::tests::the_workspaces_action_asks_for_the_ui_page_rather_than_toggling`,
  verified failing against a toggle arm), but `frame.rs`'s four-line
  follow-through has no harness in that module — `SdroxideApp` needs an
  `eframe::CreationContext` and a `RadioController`.

## The band/mode window as built (2026-10-10) — SDRuno's RX-control band panel

**Why it exists.** The docked column cannot be wider than ~280 pt: the operating
panels below it are built for ~680 pt, so a selector showing its modes in five
columns would squeeze them and overrun (the #643 guard). A window of its own can
be as wide as the operator likes with the operating panel keeping its full width
— which is SDRuno's shape anyway, and the operator's "SDRuno cloned workarea".

- **`DetachableModule::BandMenu`**, appended (slot 4), app-id
  `sdroxide-bandmenu`, window title *band & mode*, opening at **560×760** — wide
  enough for `mode_chip_grid`'s five columns, which the docked column never could
  be. `min_size` is the shared 360×240; the grid falls back to fewer columns, and
  then to a wrapped row, as the window narrows.
- **One body, two surfaces.** `show_detached_module`'s `M::BandMenu` arm draws
  the same `band_menu_body` the docked column draws, in a scroll area, so the
  window and the dock cannot come apart. At this width the mode sections lay in
  **five columns** — `the_band_window_lays_the_modes_in_five_columns` pins it
  (via the pure `mode_grid_cols`), and that the docked column is fewer.
- **`band_menu_detached()`** is the whole predicate — undocked **and** focused.
  Unlike the spectrum modules it needs nothing else present: a selector is always
  meaningful, so there is never an empty window to explain.
- **The entry points**: a **⇱ WINDOW** chip in the band popup beside **DOCK**, and
  the same on the docked column's header. The strip's **🪟 WINDOWS** menu
  (`module_window_name`) and Settings → UI (`DetachableModule::ALL`) pick it up
  by themselves — the registry-safety tests already cover the extra slot.
- **One selector on screen.** While the window is out the docked column is not
  drawn (`band_dock_panel` returns early), matching the one-owner rule SP1 and the
  3D window follow. Opening the window clears `band_docked`, so the two never
  fight.
- **Workspaces** include it automatically (`DetachableModule::ALL`); closing the
  window docks it again (`handle_detached_outcome`).
- **Native-only** (a window is a desktop surface); the phone keeps the popup, and
  the wasm check is unaffected.
- **Not seen in a running window** — the detached viewport no-ops under the
  headless harness, so the arm's own draw is untested here; the registry, the
  predicate, the menu name and the five-column rule are.

## The look is the **structure**, not the theme (operator, 2026-10-16)

Asked to "make a start on the SDRuno look", I read that as the visual style and
built a palette — SDRuno's steel blue-grey, on branch `fork/sdruno-look`
(`4f9f4bc1`). **The operator's answer, in as many words: "it is the structure
100%".** So the palette is not the job. It is a real theme and it can stay on
that branch, but do not treat it as progress toward the look.

**What "the structure" means, read off the one SDRuno screenshot on this machine
(`~/Afbeeldingen/sdruno.jpg`)** — these are the candidate differences, and the
next session should get him to point at which he means rather than build all of
them:

- **A captioned button matrix per control group.** SDRuno's RX CONTROL is rows of
  a caption with a dense grid of small rectangles under it — `MODE` over
  AM/SAM/FM/CW…, `DIGITAL` over BPSK…, `FILTER` over 6000/5000/…, `AGC`, `STEP`,
  `NR`, `NB/AN`, `VOLUME`, `RF GAIN`. Ours are **chips in rows**, which are
  larger and carry more text. The band menu already moved part-way with
  `mode_chip_grid` (fixed-width cells, `MODE_GRID_COLS = 5`), so this is the
  direction that has been started.
- **Every window names itself and the program in its own title bar** — *SDRuno RX
  CONTROL*, *SDRuno MAIN SP*, *SDRuno SCANNER*. Our tool windows carry a title,
  but not the program, and the identity is in the strip rather than the window.
- **A vertical stack of function buttons down the left of MAIN** — OPT / SCAN /
  SCHEDULER / SP1 / SP2 / RX — rather than chips along a top bar.
- **Tabs inside a window** (SETT / MISC / MAIN across the top of MAIN), which is
  a different thing from our top-level radio tabs.
- **Windows are free-floating and each carries its own chrome**, not docked
  panels. The window set is built; what is not is the density and the identity
  inside them.

**Do not build all five.** Ask which. The handover's own TODO is complete, so
this is a new direction on top of finished work, and the difference between
"denser controls" and "every window announces itself" is a session each.

## House notes for whoever picks this up

- The wire type is untouched by all of this: `UiSettings` is local `config.toml`.
  A new `DetachableModule` variant is fine; a new **tool** id is fine. Only a
  `ClientScreen` field or a new `LayoutMode` variant needs a `PROTO_VERSION` bump.
- The module registry: a new docked module is a `DetachableModule` variant, its
  `detached_spec`, a `show_detached_module` arm, and a `*_window_wanted`
  predicate where it is not always present.
- A new tool: extract its body and call `self.tool_window(ctx, id, title, size,
  open, |me, ui| …)` from its `*_window` method.
- Read `UNDOCKED-HANDOVER.md` for the machinery and the SDRuno/Niri window-rule
  caveat; read `AGENTS.md` for the house rules.
