# TODO — SDR Oxide Brown

What this fork is working on. Kept here so it survives a session and is visible
to everyone on the repo. Last updated **2026-10-12**.

Upstream issues are the *maintainer's* queue; the items below marked `#NNN` are
upstream feature requests **this fork** may pick up, not a promise to upstream.

## Now / next session

- **Check our open upstream PRs for maintainer replies** — answer only where he
  asked something. Current: **#648** (eQSL QTH nickname), **#660** (UTC + local
  clock), **#661** (output-audio routing), **#663** (HFDL custom frequency chips).
- **#257 — undocked mode / separate windows on a second monitor.** In progress;
  the SDRuno window set. See `SDRUNO-UI-HANDOVER.md` and `UNDOCKED-HANDOVER.md`.

## In progress / parked

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
