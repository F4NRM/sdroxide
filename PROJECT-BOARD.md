# SDR Oxide Brown — project board content

Three columns. Achieved → **Done**, Open → **In Progress**, Roadmap → **Todo**.
Scripted via `gh project item-create` + `item-edit` once the `project` scope is
granted (`gh auth refresh -h github.com -s project`).

## Achieved (Done)

- **FT8/FT4/FT2 session ignore list** — a `−` per decode row mutes a station for
  the session, at decode ingress
- Release **v1.9.6_brown** (2026-09-29) and the pre-release **v1.9.6_brown.experimental**
- Release **v1.9.5_brown**, **v1.9.4_brown** (Brown rename), nightly builds
- **ALE** receiver core + front end + TX primitive (experimental); upstream draft PR #598
- **JTTY** receive and transmit (experimental, bench-confirmed)
- **Wide CB callsign grammar** (experimental, bench-confirmed)
- **FT8 checkpointed signal subtraction** (weak signals under strong neighbours)
- **digi-mode editable macros** (upstream PR #596, ported to the fork)
- **FSK441** decoder (upstream #555) + transmit (PR #561) and the empty-box/reply fixes
- **Panadapter "levels are hiding the picture" hint** + FIT
- **Auto-record silence split** review fixes (hidden-tab gate, own-stop, Stop-after)
- **Band openings** warm-up/dedupe fix
- **SWL report pre-fills** from the schedule on + NEW
- **Spoken alerts**: phrase preview + SAY
- **Morse trainer**, **CW key** (Settings → CW, USB paddle), **station profiles**
- **Tabbed band/mode menu** (LISTEN/OPERATE, ALL, dock), **SIG ID** guide
- **SWL**: reception log, QSL/report tracking, CSV/ADIF export, filters, schedule,
  local solar time, ECSS, receive tone, replay, scheduled recordings, band scanning
- **Grey line** on the flat maps, **meteor calendar**, **IBP beacons**, **Kp history**
- Decoders landed: **ACARS, HFDL, DSC, UVPacket, NAVTEX, HD Radio, AIS zoom, QO-100**
- **Nine upstream PRs merged** into upstream on 2026-09-28 (#583, #588, #590, #591, #593, …)
- Upstream fixes taken: WEFAX auto start/stop, RADE RX reporting, HD-on-AM, Icom WFM,
  PureSignal gate, TX drive ceiling, LimeSDR Mini, HFDL lane rate
- **Antenna rotator** — a manual **ROTATOR** window (compass, live hardware bearing,
  drag/click to point, DX-country lookup, STOP/PARK/AUTO) with an authority model so a
  manual point is not overridden by the satellite lock; point-at-DX from a decode row's
  **BEAM** chip and click-to-point on the map; **EasyComm II / GS-232** serial transports
  beside the Hamlib `rotctld` client (serial paths not proven on hardware)

## Open (In Progress)

- **Support queue, 2026-10-04.** What the fork's testers reported and where it
  stands. **Both channels are live** — the fork's issues and its discussions
  are both open, and a report arrives as an issue about as often as a
  discussion (this line said issues were disabled until 2026-10-12; it was
  wrong, and it cost a session in which nobody checked).
  - **Discussion #5 — Olivia decode (kevin2008-01): RESOLVED.** Find the fault
    and fix in MORNING.md §1. In 1.9.16 the decoder decodes real Olivia; a
    known-text sample decoded by the reporter (Wikipedia's 32/1000) reproduces
    here and is pinned as an `#[ignore]`d test
    (`SDROXIDE_OLIVIA_1000_32`). **Still open, and not a code matter:** nobody
    has copied *our own* Olivia over on fldigi/MultiPSK — the transmit half is
    unverified on air. Decode is confirmed; transmit awaiting a listener.
  - **Discussion #5 — three store/UI bugs (kevin2008-01): OPEN, unverified.**
    All separate from Olivia and not yet fixed:
    - **CLEAR RX appears to do nothing on Olivia.** Wiring is correct
      (`clear_rx_chip` → `DigiClearRx` → controller `clear_rx` → `text_rx`);
      most likely it was a symptom of the 1.9.15 decoder producing no text.
      Re-test on 1.9.16+; still broken with text on screen ⇒ real, check the
      web-session path. **Path re-checked 2026-10-12 and confirmed complete
      end to end:** `clear_rx_chip` → `Command::DigiClearRx` → the engine's
      generic `d.clear_rx()`, which Olivia reaches through
      `TextModemController` like every other keyboard mode, and the engine
      calls `emit_digi_status()` immediately so the window empties without
      waiting for the next poll. Nothing to fix here; it needs **Kevin's
      re-test**, not code.
    - **Frequency memories capped at 3 in a new web session** (10 saved → 3
      survive). **Re-investigated 2026-10-12 and NOT reproduced.** The whole
      path was traced — `EditMemory` is keyed on the channel's **id** rather
      than its position, the engine edits in place, the server clones
      `latest.memories` per session and replays on connect, and the client
      applies it unconditionally (`frame.rs`: `RadioEvent::Memories(m) =>
      self.memories = m`). Commands travel whole through
      `ClientMsg::Command`, so there is no per-field mapping where ten could
      become three. **No cap found anywhere.** The sibling bug on this list —
      the decimal frequency being rounded — turned out to be **already fixed**
      (`ea89c7e6`, issue #138: the editor uses six decimals so it stores what
      it shows), which is a warning that this entry is stale rather than
      accurate. Reported back to kevin2008-01 in
      [discussion #18](https://github.com/madmedicnl/sdroxide-brown/discussions/18#discussioncomment-18890294)
      asking whether it still happens on a current build, and whether the loss
      is in the MEM window only or in `memories.json` too. **Do not write a
      fix against an imagined mechanism — wait for his answer.**
    - **A decimal frequency is rounded** (7.038.5 MHz → 7.037). `MemoryChannel
      .freq_hz` is `f64`, so the save or display is rounding it.
  - **SSTV receive (operator, this bench): FIXED in 1.9.17.** The low end
    (200–294 Hz) was cancelled to a dead band by the auto-notch, which was
    running on the image/tone modes; `auto_notch_applies` now excludes SSTV,
    SSTV-FM, RIFP, WEFAX, Hell, RF Paint. Separately, "hears a clear signal but
    does not decode" was an **overloaded RSP1** (a keyed CRT inches away) —
    RF-side, not code; the panels now warn when the front end is overloading.
  - **NAVTEX "jibberish" (Cal, via the operator): support advice sent.** NAVTEX
    can only decode when the dial is set to **channel − 1.7 kHz, USB** (518 →
    516.300) and a station is in its slot. The panel hovers and manual now say
    both; no code fault found. Awaiting Cal's result.
  - **Discussion #8 — the phone is the control, and it settles what the fault
    is NOT.** A phone held to the speaker decodes everything, and a phone is
    clock-independent: it just decodes the sound it hears on its own clock. So
    the RF and the audio reaching the operator are good; do not re-litigate RF,
    antennas or audio levels.
  - **Discussion #8 — BENCH RESULT (2026-10-04, decisive): the server engine
    decodes fine, with and without a client.** Instrumented `poll_digi`
    (`DIGIDECODES` log), ran the 1.9.17 build as `--server` FT8 on 14.074:
    - **no client attached:** 25 decode batches (6/13/3/16/3…stations), no
      panic.
    - **native client attached** (`--connect 127.0.0.1`): 26 batches,
      **280 stations**.
    So the **fork's server-side decode path is not the fault** — not the
    engine, not the tap, not the relay to a native client. Kevin's failure is
    **specific to the browser client** (WASM page load / WebAudio / the WASM
    relay) or to his environment — which matches his own clue (minimise →
    sound cuts → reload/lag). **Next and only candidate: does the bug appear
    in the browser on *upstream* sdroxide too** (question posted to #8). If
    upstream's browser is fine and ours is not, it is a fork browser-client
    regression; if upstream is also bad, it is shared upstream + his setup.
  - **Discussion #8 — FT8/FTx decode, REOPENED by kevin2008-01 (2026-10-04
    16:41).** "FTx is not fixed in 1.9.16." New detail: with a brand-new web
    session it still decodes almost nothing; his phone on the speaker decodes
    everything. **New hypothesis (his, and plausible):** the web client losing
    focus / a network blip desynchronises the *server's* clock or the audio
    stream, and FTx is clock-critical. Two concrete claims to chase:
    (a) minimising the browser to the taskbar cuts the sound, and restoring it
    reloads and leaves a waterfall/audio lag; (b) any power interruption
    "throws off the clock". **The server-side decode path was fixed (the
    subtract panic) but Kevin's case is still failing, so the panic was not the
    whole story — or not his story.** Next: ask him for the exact build
    hash in the `1.9.16` screenshot, and whether a **native `--connect` client
    to the same server** decodes (that separates "server engine" from "browser
    relay/timing").
  - **Discussion #9 — phone (web) layout: FIXED in 1.9.19 (`b65cda82`), and the
    two theories behind it were both wrong.** Read the screenshot before
    theorising; that is what settled it.
    - **It was the split view, not the layout.** With three radios open on a
      360 pt phone, `MultiApp::ui` split the main area into equal columns with
      no tier check — `(360 − 12) / 3 = 116 pt` each. Not three radios: the
      phone layout clipped to a sliver, frequency readout truncated, S-meter
      unreadable, one radio name wrapped to a single letter per line. His
      "layout is not centred" is the same single fact. The split is now not
      drawn on the phone; the focused radio takes the window and the stored
      panes are kept, so widening gives the split back.
    - **Two things it was NOT.** (a) *A missing spectrum on a phone is not a
      bug* — `layout::Tier::waterfall_only()` is true for `Phone` and has been
      since **2026-07-31**, long before 1.9.17; suppressing it is deliberate
      ("a spectrum trace in a 360 pt window costs the waterfall a third of its
      height"). (b) *`digi_pane` defaulting to 0* is irrelevant, since the
      phone never draws a spectrum at any pane. Two hours of plausible theory
      would have shipped a wrong fix.
    - **The guards that should have caught it never ran.** Both phone tests
      dropped egui's texture delta unapplied → debug-assert → the whole
      `sdroxide-ui` **debug** suite was red (697 passed / 2 failed) with the
      failure in the harness, not the code. And `phone_crash_regression_1440x3200`
      fed the *physical* pixel count as logical points: 1440 pt wide is the
      **Desktop** tier, so under a phone-sounding name it guarded nothing
      phone-shaped. Now `411x914` (his panel at DPR 3.5), and the helper
      **asserts the tier resolves to `Phone`** so that mistake cannot recur
      quietly. Suite is 704/704 in debug *and* release.
    - **Two repair tests, each verified to fail when its own rule is reverted**
      — `split_plan` (the split is not drawn on a phone) and `strip_set` (the
      other radios stay *reachable*; reading the stored panes would grey them
      as "already in another column", leaving a phone that can look at one
      radio and not change which).
    - **NOT verified on a device.** No `MultiApp` harness here (`new` needs an
      `eframe::CreationContext`), so the rendering is unproven — **Kevin's
      reload is the test.** Not browser-tested either.
    - **Still open: the 1.9.17 crash itself.** Never seen on the bench; the
      layout fault is fixed but the crash is not, and the changelog says so.
      Console output if it recurs.
    - **Older note on this entry, kept because it was the wrong assumption:**
      the base layout fault is shared with upstream (dividebysandwich#516) and
      his screenshot shows it on the official build too. That may still be true
      of *upstream's* layout, but our half of #9 was ours and is now fixed —
      do not file the whole report upstream again.
    - **SUPERSEDED — the old theory, kept so it is not re-derived.** It blamed an
      over-stuffed `Tier::Phone` chip row (GRID, SIG ID, ISM/ISL, ENIGMA, HFDL,
      the listener chips) and a possible overflow / divide-by-zero on
      `frame.rs:675/923`, `top_bar.rs:664/704/1473/6917` or the `phone_pane`
      splits. Reading the screenshot instead of theorising showed the cause is
      neither: nothing overflowed a row, it was **three 116 pt radio columns**.
      The 1.9.18 phone work (`e8e7812a`, three chips + a ☰ menu) had already
      fixed the chip row; this release fixed the split. **Do not audit the chip
      row for this report.**
  - **Discussion #7 — SSTV (kevin2008-01).** Three asks: (1) menu extra
    to **re-upload**, **send to** (email a QSL picture) and **save to** a chosen
    location on a received picture; (2) **CTR does not centre** in SSTV
    (1.9.10–15) — frequencies stay on the left; (3) received pictures and the
    **SAVE** button only appear after closing and reopening the session —
    images are not refreshed within a session. (3) is the same shape as the
    screen-settings sync area; treat as a real UI/state bug.
    - **(1) re-upload: DONE, `112d8df1`, in 1.9.19.** A **Re-upload** chip beside
      **Save picture…** loads the picture into the selected transmit slot. It
      stops there — TX is still a separate deliberate press — and is greyed
      with a reason on a receive-only radio.
    - **(1) save-to: already shipped.** `Save picture…` opens a real file
      dialog (`download::save_as` → rfd). Open question for the operator:
      **rename it to "Save picture as…"**, since the wording does not say it
      will ask. Not done unilaterally — raised in the #7 reply instead.
    - **(1) send-to: NOT BUILT, answered instead.** No mail account to send
      from, and a browser cannot attach a file to a message. The only thing
      shippable is "save it and open your mail client", which is the operator's
      own two clicks wearing our hat — refused under the house rule against a
      control that silently does nothing. If asked again, this is the reason.
    - **(3): already fixed in 1.9.17** by `d9425662` — the ask now follows the
      picture's *bytes*, not the full-size texture. Kevin's "therefore images
      are not refreshed" was his own inference from the SAVE symptom; the
      gallery is live via `on_saved`/`on_listing`.
    - **(2) CTR: probably already fixed — waiting on a retest.** CTR is
      `center_on_vfo`, the DISPLAY chip that keeps the dial in the middle of
      the panadapter. Kevin reported it against **1.9.10–15**, and `8c0aee9d`
      ("choosing SSTV lands on that band's SSTV frequency", 2026-10-01, in
      1.9.18) very likely fixed it: 80 m's SSTV frequency is **3.7300**,
      which is the middle of the 3.7275–3.7325 window in his screenshot, and
      before that commit choosing SSTV left the dial wherever it was. Asked him
      to retest on 1.9.19. **Do not patch this speculatively** — if he says it
      is still off-centre, the thing to check is whether SSTV's `AudioCursor`
      anchor (`center_on_cursor`, from `holds_standard_tones()`) fights
      `active_freq_hz`.
    - **(4) weak-signal decode** is upstream PR #587 / issue #622. Not ours.
- **Upstream PR #626 (CW engine-side keyer) — NEXT SESSION (queued 2026-10-04).**
  Review received 2026-10-03; branch `upstream-pr/cw-key-engine`. Do it in this
  order: **(2) the disconnect-safety fix first**, then (3) disarm, then the
  rest, with the A-vs-B squeeze test. Work
  needed.** Maintainer: shape is right (keyer in the engine, contacts not
  edges, appended wire), but the **lifecycle** is wrong:
  - a paddle press from idle never starts an over (`set_cw_contacts` doesn't
    key TX the way `key_down` does — only works once `fill_tx_block` runs; the
    tests hide it);
  - **a held paddle is not released when the client disconnects** (session
    cleanup sends only `CwKey(false)`, and the keyer skips the hold cap — must
    fix before it keys a transmitter unattended);
  - the keyer is **never disarmed**: `abort_tx`, `set_straight(false)` and
    config changes leave it running; after one paddle use the straight key goes
    dead; on a CAT rig it is armed *before* the refusal;
  - **mode B does not send the trailing element** when a squeeze is released
    mid-element — add a squeeze-release test pinning A vs B;
  - taps are latched only in `poll`, so press+release in one batch is lost;
  - the code/out text buffers grow unbounded **on the audio thread**.
  These are the next work item on this PR (it is a draft, so no rush, but they
  are real).
- **ALE**: decode a real burst off-air; wire TX; fold the mode into PR #598
- Upstream PRs awaiting review: **#537** band openings, **#545** (tr)uSDX nG,
  **#554** UVPacket, **#557** rec silence, **#559** band-menu captions,
  **#561** FSK441 TX, **#568** Morse trainer, **#569** CW keyer,
  **#572** CW no-link fallback, **#573** CW key package, **#586** FT8 SIC,
  **#598** ALE
- **Rebase #568 and #561** onto current `upstream/main`
- ALE experimental release: publish/finish once the live decode works

## Known-red tests and unverified claims

- **`skim_window` — genuinely red, and the fault is the skimmer's CW
  *decode*. Next diagnosis, not a release-day patch** (measured 2026-10-05).
  `sdroxide-radio --test skim_window`,
  `a_station_on_the_visible_part_of_a_wide_span_is_skimmed`, fails **alone and
  in a full run**, identically on `v1.9.16_brown` / `v1.9.17_brown` /
  `v1.9.18_brown` — so it predates every commit in 1.9.19 and is not a
  regression from any of them. It is in 1.9.19's changelog under "Not fixed"
  so whoever reads the release sees it too.
  **What the failure already rules out:** the station on 14.060 MHz is spotted
  hundreds of times at the right frequency, so the window-following this test
  exists to check **works**. What never arrives is
  `callsign == Some("W1AW")`. The fault is CW decoding inside the skimmer —
  not the view, the span, or the spotting.
  **What it needs:** `keyed_cw` builds a real `CwTx` envelope and interpolates
  it, and there is **no off-air recording to check the decode against**. So the
  first job is to find what a genuine CW signal produces through this path, not
  to loosen a threshold until it goes green.
  **Do not** "fix" it by weakening the assertion, or by treating it as the
  flake it was long mistaken for — an earlier `AGENTS.md` note claimed it
  "passes alone", which was false; both files now say so.

## Session ignore list — design (coded, 2026-09-29)

**Why session-only:** a memory `HashSet` with no file, no wire, no schema is
cheaper to run and to build than a persisted list, and avoids permanent hiding.
Familiar `−` UX without the permanence. Keep the filter as
`session_ignored ∪ (future persisted)` so persistence can be added later.

**As built (fork-only, UI layer):**
- `SdroxideApp` gains `session_ignored: ignore::Ignored` (a `HashSet<String>`,
  init in `app/mod.rs` alongside `digi_status`). No `PROTO_VERSION` change.
- `app/ignore.rs` holds the whole rule: `key` (trim + upper), `is_ignored`,
  `toggle`, `retain_unignored`, and `toggle_ignore` / `clear_ignored` /
  `ignored_hover`. Six unit tests.
- Decode row: a `−` chip beside the queue `+`/`＋` chip (`panels/decodes.rs`),
  drawn for any decode naming a sender, so it works for listeners too. The
  press is staged and written after the row loop (which holds borrows).
- **Filter at ingress**, in the `RadioEvent::Ft8Decodes` arm in `frame.rs` —
  before the announcer, the audible alerts, the decode list and the propagation
  field, so one rule covers all of them. Empty batch → `continue`.
- **Mute everything** also asked in `auto_target`, because rows held *before*
  the press are still in `digi_decodes`; without it auto mode would answer the
  very station just muted. Spotting/upload untouched.
- **Undo/visibility**: an `N ignored · clear` chip in the decode header, drawn
  only when non-empty. Muted rows are dimmed (`ROW_BG × 0.45`, grey accent,
  badge `MUTED`) and age out, so a mis-click is reversible and nothing refills
  the space.
- Glyphs are `−` / `×` — both already load-bearing in `sdroxide-ui`; a
  pictographic "no entry" sign was the one glyph at risk of the font atlas not
  carrying it.


## Roadmap (Todo)

- **Performance pass** — a full optimize run over the per-frame paths, not
  started. Nothing measured is *known* to be slow; the point is to find out
  where the time actually goes rather than to guess. Where to look, and what is
  already ruled out, so the next session does not re-derive it:
  - **The decode-list row loop** (`app/panels/decodes.rs`, 200 rows/frame) is the
    prime suspect: per row it clones the sender for `who`, builds several
    `RichText`s, and runs two layout passes. This is the heaviest per-frame
    allocation site in the UI.
  - **The session ignore list is NOT a problem** — measured 2026-09-29 in
    release: `is_ignored` over all 200 rows costs **7.6 µs/frame (37 ns/call)**,
    0.05 % of a 60 fps frame, and 3.4 µs of that is the `to_ascii_uppercase`
    `String` the row already pays for `who`. **Do not "optimize" it**; a
    `HashSet<String>` was chosen over `&str`/`Cow` deliberately — the set is
    session-sized and the clarity is worth the allocation.
  - Other suspects, unmeasured: `log_index()` (rebuilds when the log length
    changes — check it is not rebuilding per frame), `auto_target`'s `live` Vec
    clone (only when the ignore set is non-empty), the waterfall/spectrum
    repaint, and the prop/map texture rebuilds.
  - Method: `cargo test --release` with a `#[test]` + `Instant` harness
    (`std::time::Instant`, no criterion in the tree) run under
    `--release` — a debug-build measurement of a path this hot is meaningless.
    Beware the repo's `rustfmt.toml`: a bench harness left in a source file
    must be removed or it will not compile.
- ~~**DAB/DAB+** — blocked on `xoolive/desperado#52`~~ — **no longer blocked, and
  no longer ours.** Shipped and confirmed on the air by Kevin (2026-10-11): a
  PlutoSDR at 2.5 Msps decodes DAB+ cleanly. He owns it and has the bench, so it
  is off our list — do not pick it up.
- **M17** (RX-first) and **ALE Selcall**
- **DSC** audio front end (protocol+framer done; detector needs a real burst)
- **Inmarsat** (wideband-lane decoder, upstream #187)
- Decoder candidates: **POCSAG/FLEX**, **ARDOP**, **FLARM/OGN**, **UAT 978**
- Listener: **D-RAP absorption map**, **azimuthal map**, solar-wind/Bz sparklines,
  solar-cycle chart, broadcast noise reduction
- **Phase 4 polish**: say why the dial is not the frequency you picked; UTC everywhere
- **CW reception reporting** to PSK Reporter
