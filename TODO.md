# TODO — SDR Oxide Brown

What this fork is working on. Kept here so it survives a session and is visible
to everyone on the repo. Last updated **2026-10-15**.

Upstream issues are the *maintainer's* queue; the items below marked `#NNN` are
upstream feature requests **this fork** may pick up, not a promise to upstream.

## Now — pick one

1. ~~**The two-window render harness.**~~ **DONE.** `a_tool_window_does_not_paint_above_the_window_stack`
   drives two tool windows at once and asserts **no layer above the window
   stack exists**, which is the mechanism the ⇱ bug lived in. Verified failing
   on the old code, naming both chips: `[Foreground …weather…title-btn,
   Foreground …known-calls…title-btn]`. The property is the mechanism rather
   than 'the lower window is hidden', because the latter needs a real pointer
   and a compositor to mean anything; this fails the moment anyone reaches for
   `Order::Foreground` in a window again.
2. **UDP audio out (#594)** — built, socket-tested, **never listened to**. Stream
   it to `aplay` on the bench and confirm it is audio and not a crackle. The
   operator's machine, two minutes.
3. **The tool window's ⇱/⇲ and undocked mode (#257).** The SDRuno window set —
   the largest thing here, and in progress.
4. **`docs/synop-notes.md`** should keep being updated rather than written once
   and left. The two wrong "fixes" it records are what a third reader would
   otherwise repeat.

### Waiting on someone, and nothing to do about it

- **SYNOP feedback (#88)** — BETA, built, tested against four real messages.
  **Nobody has decoded one off the air.** When Kevin reports: either the BETA
  marks come off or the decoder gets fixed.
- **S-meter jumps with the mouse** (#69) — the question is asked on the thread:
  does the *number* jump, or only the needle? bebepip has not answered.
- **Grey-line gradient** — built, verified only on a path entirely in night.
  Needs one run over a **partly daylit** path, which is the case the feature
  exists for.

### Owed, and not urgent

- **Two things are in no release yet.** The Bluetooth microphone, the window
  title, PSTRotator, the SYNOP decoder, the ⇱ fix and the #537 fix are all
  **only on `main`**. The next release carries them; 2.0.12 carries none.

### Done 2026-10-15, for reference

- **⇱ chip** (`3f877e30`) — `Order::Middle`, not `Foreground`.
- **#537 false OPEN alerts** — the warm-up clock no longer survives the dial,
  and a spot no longer starts one.
- **SYNOP/SHIP manual section** (`62c7a9c0`) — §3.3a.
- **Weather quick-start in four languages**, with PDFs.
- **Branches pruned** — 38 deleted, `.git` 417M → 339M.

## Done 2026-10-15

- **SYNOP/SHIP weather reports** — see *Now / next session*; **BETA**, #88 open.
- **Bluetooth microphone** — solved, in tonight's nightly.
- **Upstream takes** — the window title (#664), PSTRotator (#638); FT8 #667 was
  **already ours** and was skipped rather than forced.
- **`sdrbrown.info`** — the site is up and waiting on GitHub's certificate.
- **Issue template** — now a form, with the version **required**. We had one
  already and it failed three ways: last in the list, optional, and its example
  said `v1.6.0` — *upstream's* scheme.
- **S-meter calibration dropdown removed** — it was built on a reading from the
  wrong build. Upstream has its own (#642), so ours was redundant as well as
  wrong-premised.

## Done 2026-10-14

- **Search by frequency in SCHEDULE** (`2afdfce8`) — bebepip's ask on #69. `6185`,
  `6185 kHz` or `6.185` find their transmissions. **A search is forgiving where
  the import must not be**: with no unit, `153` is longwave *or* 153 MHz, so both
  readings are tried and the range rule drops the impossible one. A name is never
  read as a frequency, pinned by a test.
- **House rules: "Needs feedback", not "Not proven"** (`8fe77c96`). The last
  heading is a **request**, not a disclaimer — write each line as the ask.
- **House rules: replies are prose** (`33007361`). From the Kevin meeting: answer
  to the point, easy on headings and bold, polite and personal. Markdown is for
  the repository; a reply is a message.
- **Replied on #69 and #14**; recorded **DiamondRadio** in AGENTS.md (freeware,
  source private — RadioScript's answer again, and the case is thin for the same
  board and the same no-I/Q ceiling).
- **`BT-MIKE-HANDOVER.md`** — the Midland Bluetooth mike, **solved on main**: a
  PipeWire/Bluetooth device now appears in Settings → Audio by name and can be
  picked, with a warning saying it takes over the machine's recording. Verified
  on the operator's DUALMIKE. In tonight's nightly, not yet in a release.

## Done 2026-10-13

- **Upstream take, 4 items, all verified.** Checked all 69 upstream commits; most
  are features we already built ourselves (the reason `git cherry` matches none of
  them). Taken:
  - **SSTV decoder, whole series** (`3a4442f4`) — a **drop-in**, not a merge: the
    workspace compiled unchanged and upstream had already caught up with the
    fork's Wraase/PD/SC-2/FSK-ID work. Measured on their own SNR harness: error
    87.2→75.6 at 0 dB, 42.4→33.7 at 10 dB, **slightly worse above 30 dB**
    (4.3→5.5) — the trade is stated in the commit.
  - **FT8 #667** (`4ba137bb`) — stream decodes as found, skip the slow pass when a
    slot is waiting. A merge: our `DecodeJob` enum and `set_cb_wide` kept, their
    streaming and accumulated drain taken.
  - **CW #625** (`2cabad8b`) — PTT holds the transmitter for the keyer. Additive
    conflicts, both sides kept.
  - **RTTY #576** (`46a42f0b`) — applied **clean**, so it is complementary to our
    own anchor fix (`c26b7b82`), not overlapping. The open question from the last
    survey is answered.
  - Gate: **5762 passed, 0 failed**.
- **Still open upstream, lower value, not taken:** `4fdcce19` AIS LO offset,
  `065249bf` HFDL USB-not-FM (we carry both lanes), `763a961f` globe ring height +
  map non-finite recovery, `edfb0343` tab-strip wrap (we chose scroll).
- ~~**CHANGELOG 2.0.10 entry is still long**~~ — **FIXED.** Hand-edited to 45
  lines; 2.0.11 is 31 and 2.0.12 is 33. The guard is now a **hard fail in
  `release.yml` at 60 lines**, because the short-notes rule had been written
  down, read, and broken three times and only a check stops that.

## Now / next session

- **#594 UDP audio out — BUILT, fork-only** (`b0271a88`). `RadioConfig::udp_audio`,
  `host:port`, streamed from the **recording tap** so the volume, MUTE and
  transmitting do not affect it; mono 16-bit, one datagram per block.
  `PROTO_VERSION` 207 → 208. The upstream issue is answered pointing here.
  **Worth a real check:** stream it to `aplay` on the bench and confirm it is the
  audio and not a crackle — the format is tested over a real socket, that it
  *sounds* right is not.

- ~~**#77 SYNOP / SHIP weather decoder**~~ — **BUILT, BETA** (`d1d9bc8c`), asked
  for by Kevin (F4NRM) on 2026-10-14. RTTY on 4583 / 7646 / 10100.8 kHz, SHIP at
  02:00 and 07:35 UTC, SYNOP at 00:35 and 12:10. A **WEATHER** chip opens the
  list when a report has been heard; the window shows the raw line beside the
  decoded groups. Client-side detection, **no wire change**. Gate silent, 892 UI
  and 733 types tests green.
  **Verification is the open item**, and issue **#88** asks Kevin for it: the
  decoder is tested against four *real published* messages (HKO, CTOM Oman,
  tutiempo.net, AWI) and decodes all four, but **no report has been decoded off
  the air**. When he reports, either the BETA marks come off or the decoder is
  fixed. Design and the six wrong readings that shaped it are in
  `docs/synop-notes.md`.
- **#75 the satellite picker lists the whole catalogue** — drdatapi, 2026-10-14.
  **BUILT** (`04032baf`): an **all satellites** checkbox, off by default, sharing
  one `satellite_is_workable` test with the PASSES tab so the two cannot drift.

- ~~**Check our open upstream PRs for maintainer replies**~~ — **DONE 2026-10-15.**
  **All four merged upstream: #648, #660, #661, #663** (plus #557). Our queue
  upstream is empty. Still open there, none awaiting us: #626, #597, #554, #545,
  #537, #598, #559.
  - **#537 answered 2026-10-15** — the maintainer reports the warm-up fix is
    only called from tests and that false OPEN alerts remain when returning to a
    band or when DX-cluster spots mix with PSK Reporter. Replied honestly: the
    clock *is* started in `ingest`, but the **two feeds share one tracker and one
    per-path baseline**, and a DX-cluster-only path starts its baseline at the
    moment the spot lands — so the mixing is the likely cause, not a missed call.
    **Unmeasured, and the reply says so.** Open question: does the false OPEN
    only ever involve a cluster-only path, and does it survive leaving the band
    and returning? Two questions asked, no code changed on a guess.
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
  `SWL-HANDOVER.md`. Since then bebepip reported the S-meter bug (below) and asked
  for frequency search, which is **done** (`2afdfce8`).

## In progress / parked

- **The satellite pass scheduler is in** (`4dee2184`) — a PASSES tab in the SAT
  window, taken from upstream #623. It applied clean: the pass computation
  (`sdroxide_solar::satellites::Pass`, `next_passes`) was already ours and tested,
  so this was the view over a proven calculation. **The operator wants the
  satellite side in a release soon.**
- **#657 OpenStreetMap background — PARKED, deliberately, after scoping.** The
  finding: **the map is already 8192×4096** with borders, rivers, cities and a
  vector `lines.bin`, drawn through a mip pyramid. It is not a coarse outline. OSM
  would add street-level detail at high zoom, and this fork's maps are world views
  (LISTEN, ADS-B, AIS, HFDL, propagation). Against that: our maps are
  **equirectangular** and OSM tiles are **Web Mercator**, so every tile needs
  warping; the UI crate has **no network path at all** today, so it needs a fetch
  worker, disk cache, rate limit, User-Agent and attribution; and it has to work
  in the browser build. **A fetch stack is not the right trade for a map that is
  already this good.**
  **Kevin (F4NRM) is now in charge of layout and visuals**, so this is his to pick
  up if it is ever wanted — the scope above is what he needs.

- **The S-meter jumps with the mouse while SCHEDULE or LISTEN is docked** —
  reported by bebepip on #69 (2026-10-14), and **likely a regression from the
  windows added on 2026-10-13**. *"moving the mouse anywhere within the program
  window causes the S-meter needle to jump to full scale, tracking the mouse's
  movement... if the window is undocked from the main window, the issue vanishes."*
  **What is established:** the needle reads `self.meters`, which is written in
  **exactly one place** (`frame.rs:2807`, from `RadioEvent::Meters`), so the
  needle is honest and the question is whether the *value* or the *animation* is at
  fault. The needle is a per-frame spring at `widgets/smeter.rs:946-966` — semi-
  implicit Euler, ω=26, ζ=0.75, `dt` clamped to `[0.001, 0.05]`, so ω·dt = 1.3 at
  the clamp and it **should** be stable; it keeps itself alive with
  `animate(ui)` → `repaint::after_ms(30)`, which is why a docked window's repaint
  pattern is the suspect and an undocked one is not.
  **The one question that splits it, asked on the thread:** does the *number*
  beside the meter jump too, or only the needle? Number → the engine really is
  spiking on mouse motion (a different fault, and a worse one). Needle only → the
  animation. **Do not fix from the symptom** — wait for the answer.
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

- ~~Doppler-corrected panadapter~~ — **BUILT** (`fac88c05`): downlink segments
  labelled on the frequency scale, shifted by the correction in force. Only
  downlinks; the operator's own frequency table wins over the built-in one.
- Satellite telemetry decoders (FSK/AFSK/GFSK/GMSK/BPSK) with SatNOGS submission.
- FT4 console for linear-transponder satellites.
- ~~Pass scheduler~~ — **BUILT** (`4dee2184`, the PASSES tab).
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
