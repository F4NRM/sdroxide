# TODO — SDR Oxide Brown

What this fork is working on. Kept here so it survives a session and is visible
to everyone on the repo. Last updated **2026-10-13**.

Upstream issues are the *maintainer's* queue; the items below marked `#NNN` are
upstream feature requests **this fork** may pick up, not a promise to upstream.

## Now / next session

- **Check our open upstream PRs for maintainer replies** — answer only where he
  asked something. Current: **#648** (eQSL QTH nickname), **#660** (UTC + local
  clock), **#661** (output-audio routing), **#663** (HFDL custom frequency chips).
- **#257 — undocked mode / separate windows on a second monitor.** In progress;
  the SDRuno window set. See `SDRUNO-UI-HANDOVER.md` and `UNDOCKED-HANDOVER.md`.
  - **A tool window's ⇱/⇲ chip is drawn after its window is covered.** From the
    operator's screenshots of 2026-10-13 (22:11:44 against 22:11:53, nine seconds
    apart, only the windows moved): with LISTEN on top of SCHEDULE, SCHEDULE's
    open-in-window button still paints **over** LISTEN's body, at (941, 530),
    while the rest of its title bar is correctly hidden underneath.
    **Likely cause, not yet verified:** a window icon chip is drawn for a module
    that is currently undocked — there are **seven** sites that draw one
    (`tool_window_chip`, plus six in `frame.rs` and `top_bar.rs`, which are the
    docked-module headers and the band popup's ⇱ WINDOW). The BandMenu work already
    established the rule this should follow: *"while the window is out the docked
    column is not drawn (one selector on screen)"*.
    **Do not fix this from the screenshot.** I named a cause from one before today
    and was wrong three times in a row; find which site draws it, then pin it with
    a test that two overlapping tool windows leave nothing of the lower one visible
    outside its own rect.
- **~~#69 — a second broadcast schedule source (Aoki / HFCC / your own).~~ BUILT**
  (`4b4311c2`, `98b83e07`, `6544525a`, `8fa2119e`). All three sources are in one
  **master store** with a per-row source tag and a SCHEDULE **Source filter**;
  your own list imports from CSV/text/JSON. **Aoki 3597 rows, HFCC 3363 rows with
  names, sites and positions — both verified live.** EiBi unchanged. See
  `SWL-HANDOVER.md`. What is *not* done: nothing on the roadmap — see the
  remaining notes there.

## In progress / parked

- **The grey line on a path — built, and verified by one screenshot only**
  (`88d2bd0c`, `7ed1e7a5`). Each dot on a great-circle path now carries its own
  twilight, drawn with the same ramp the flat map's night shading uses, so a dot
  and the land beneath it agree. It **fades rather than flips** — a binary
  light/dark split would lay a hard edge across the line exactly where the
  terminator is soft — and a night dot keeps a faint ring, because dim is legible
  while absent is a claim about the signal that is not true.
  **One site then drifted: the first cut gave the *selected station's* path the
  twilight and left the *per-station* paths flat — and LISTEN passes no selected
  station, so the window where the feature lives was showing an undarkened path.
  Both now go through one `twilight_dot` (`88d2bd0c`), because the drift was the
  bug: the same treatment applied twice by hand is how one site gets it and the
  other does not.
  **Open — one run by the operator:** it has only ever been seen on a path that is
  *entirely* in night ("100% of this path in night", Fuzhou → NL, 22:11 local).
  Pick a station whose path is **partly** daylit and check the gradient runs the
  right way along the line. That is the case the feature exists for and the case
  the screenshot cannot demonstrate. `dark_share` has always been unit-tested as a
  number; the *drawing* cannot be, so the window is the oracle.
- **Antenna modelling tool** — parked on branch `fork/antenna-tool` (a validated
  MoM solver: dipole, monopole-over-perfect-ground, delta loop, Yagi; `.maa` and
  `antennas.json`; 2D pattern). Next: the source-between-terminals junction
  (unlocks inverted-V / ground plane), real Fresnel ground, lumped loads. See
  `docs/antenna-tool-ideas.md`.

## Backlog — SkyRoof satellite ideas (VE3NEA; see `docs/skyroof-notes.md`)

- Doppler-corrected panadapter — satellite names + transponder-segment
  boundaries on the frequency scale.
- Satellite telemetry decoders (FSK/AFSK/GFSK/GMSK/BPSK) with SatNOGS submission.
- FT4 console for linear-transponder satellites.
- Pass scheduler.
- SSDV image rebuild across passes.
- Codec2 voice playback from satellite telemetry frames.

*(#623 fly-over prediction and much of #651 SAT lock/Doppler are already covered
by our own sat code — `sdroxide-solar/src/satellites.rs`, `satlock.rs`, the SAT
window. I/Q out over UDP is #594.)*

## Backlog — upstream feature requests (this fork may pick up)

**Fork-relevant, medium**
- ~~#617 HFDL custom frequency chips~~ — **done** in `1bddef9c`; offered upstream
  as **#663** (built on `upstream/main`, four files, no wire change).
- ~~#69 import a broadcast schedule other than EiBi~~ — **done 2026-10-13**, see
  *Now / next session* above. Kept here because the entry it replaced read
  *"scoped, not built"* and that was two days out of date.
- **#458 QRZ lookup + KML export** — the **KML export is done** (2026-10-12): a
  **KML** chip in the SPOTS window saves what is on screen for Google Earth.
  `spots_to_kml` lives in `sdroxide-types`, so the browser client writes the same
  bytes. Only the kinds and frequencies the filters have lit are exported, and a
  spot with no grid is left out rather than pinned at null island — the chip says
  how many were dropped. The **bulk/automatic lookup half is still open**:
  `lookup.rs` has single-call lookup and the per-mode grid lookups use it.
- #322 PC-keyboard as a Morse key
- #536 RX audio to the radio's own local speaker (HPSDR Protocol-1 EP2)
- #653 split screen
- #624 FTP upload
- #442 switching external band filters / transverters
- #521 IC-7610 internal ATU control
- #261 auto-tune with the rig's built-in tuner
- #270 ADS-B remote web view
- #594 UDP audio/I-Q stream out
- #480 MCP server
- #657 OpenStreetMap map background
- #229 TX interlock across radios / antenna switch / CATSync / SDR TX protection

**Smaller / hardware / large, low**
- #250 ELAD FDM-S2 · #344 Dragonfly RX-103 · #383 RS-HFIQ · #406 remote tuner
  for HL2 IO board · #448 RSPDuo + RSP1 as a third tuner · #525 Pluto Sky R2 ·
  #595 Perseus · #639 KV4P HT module · #325 band-decoder output · #327 CI-V GPS
  time/location · #571 Kenwood TS-480 CAT · #591 RGO ONE rig · #602/#605/#642
  RTL-SDR 4L / Blog v4 / S-meter trim · #227 USB relay · #289 link VFO ·
  #413 x-band repeater · #589 RPM packaging · #237 native C-driver SDR support ·
  #262 ALE (in fork, experimental) · #267 OpenWebRX · #416 multi-RX HL-2 /
  Red Pitaya · #446 Ettus B210 · #449 M17 · #652 QO-100 full duplex

**Done / covered, for reference**
- #212 NAVTEX + time-sync — done (the panel shows a message's stated time).
- #355 spectrum smoothing — covered (SPEC → Speed, "slower averages more").
- #638 PST Rotator — answered (use Hamlib rotctld, not UDP).
- #623 satellite fly-over prediction — covered by our sat code.
- #607 DMR/YSF/D-Star — decided against (the AMBE+2 patent, US 8,359,197).
- #641 SWL reception log — in the fork.
