# TODO — SDR Oxide Brown

What this fork is working on, kept here so it survives a session and is visible
on the repo. Last updated **2026-10-15**.

Upstream issues are the *maintainer's* queue; items marked `#NNN` are upstream
feature requests **this fork** may pick up, not a promise to upstream.

---

## Now

1. **Undocked mode (#257)** — the SDRuno window set, and the largest thing on
   this list. See `SDRUNO-UI-HANDOVER.md` and `UNDOCKED-HANDOVER.md`. It wants a
   session of its own rather than the tail of one.
2. **`docs/synop-notes.md`** — keep it current rather than written once. The wrong
   "fixes" it records are what a third reader would otherwise repeat.
3. **`docs/USER_MANUAL.md`** — the ⇱/⇲ chip's `Order::Middle` rule, and the two
   UDP fixes, are worth a line where a reader would look for them. Not urgent;
   the manual is for the operator, not for us.

## Waiting on other people, and nothing to do about it

- **SYNOP feedback (#88)** — BETA. Built, tested against four real published
  messages, **nobody has decoded one off the air**. When Kevin reports, either
  the BETA marks come off or the decoder gets fixed.
- **The S-meter jumps with the mouse** (#69) — one question is on the thread and
  unanswered: does the *number* beside the meter jump too, or only the needle?
  Number → a real engine fault. Needle only → the animation. **Do not fix from
  the symptom.**
- **The grey-line gradient** — built, verified only on a path *entirely* in
  night. Needs one run over a **partly daylit** path, which is the case the
  feature exists for. `dark_share` is unit-tested as a number; the drawing
  cannot be, so the window is the oracle.

## Not in a release yet

Everything below is on `main` and **in no release**. `2.0.12` predates all of it;
the next release is the first that carries it.

- the Bluetooth microphone, and the PipeWire device list
- the window title showing the version (#664)
- PSTRotator (#638)
- the SYNOP/SHIP decoder, BETA
- the ⇱ chip fix, and the two-window harness
- the #537 false-OPEN fix
- the audio stream (#594), now heard on the bench

## Parked, with the reason

- **Antenna modelling tool** — branch `fork/antenna-tool`, a validated MoM solver
  (dipole, monopole-over-perfect-ground, delta loop, Yagi). Next would be the
  source-between-terminals junction, real Fresnel ground, lumped loads. See
  `docs/antenna-tool-ideas.md`.
- **#657 OpenStreetMap background** — parked after scoping. The map is already
  8192×4096 with rivers, cities and a vector overlay; our maps are
  equirectangular and OSM tiles are Web Mercator, the UI crate has no network
  path at all, and it has to work in the browser. **A fetch stack is not the
  right trade for a map that is already this good.** Kevin owns layout and
  visuals, so it is his to pick up if it is ever wanted.
- **Satellite telemetry decoders** (FSK/AFSK/GFSK/GMSK/BPSK) — the ISS is
  **already covered** (AFSK + AX.25 packet, interoperable with DireWolf), and the
  Fox/QO-100 side is BPSK, which `sdroxide-qo100` does. What is left is
  per-mission field layouts, each needing a published table and a real capture.
  See `docs/skyroof-notes.md`.
- **FT4 console, SSDV across passes, Codec2 playback** — SkyRoof ideas, not
  scoped.

## Backlog — upstream requests this fork may pick up

**Fork-relevant**

- **#458** — the **KML export is done** (a KML chip in SPOTS). The bulk/automatic
  QRZ lookup half is open.
- **#322** PC-keyboard as a Morse key · **#536** RX audio to the rig's own local
  speaker (HPSDR P1 EP2) · **#653** split screen · **#624** FTP upload ·
  **#442** external band filters / transverters · **#521** IC-7610 internal ATU ·
  **#261** auto-tune with the rig's tuner · **#270** ADS-B remote web view ·
  **#480** MCP server · **#229** TX interlock across radios

**Smaller, hardware, or large and low**

- **#250** ELAD FDM-S2 · **#344** Dragonfly RX-103 · **#383** RS-HFIQ ·
  **#406** remote tuner for HL2 IO · **#448** RSPDuo + RSP1 as a third tuner ·
  **#525** Pluto Sky R2 · **#595** Perseus · **#639** KV4P HT · **#325**
  band-decoder output · **#327** CI-V GPS · **#571** TS-480 CAT · **#591** RGO ONE ·
  **#227** USB relay · **#289** link VFO · **#413** x-band repeater · **#589**
  RPM packaging · **#237** native C-driver SDRs · **#267** OpenWebRX ·
  **#416** multi-RX HL-2 / Red Pitaya · **#446** Ettus B210 · **#449** M17 ·
  **#652** QO-100 full duplex

**Done or covered, kept for reference**

- #212 NAVTEX time-sync · #355 spectrum smoothing · #594 UDP audio · #617 HFDL
  frequency chips · #623 satellite passes · #638 PSTRotator (**taken from
  upstream**, not merely answered) · #641 SWL reception log · #642 S-meter
  calibration (upstream has its own; ours was removed) · #648/#660/#661/#663
  (all merged upstream).
- #607 DMR/YSF/D-Star — **decided against**: the AMBE+2 patent, US 8,359,197,
  until 2028-05-20. See AGENTS.md before answering anyone again.

---

## Recently done

### 2026-10-15

- **Audio stream (#594)** — heard on the bench: *"perfect replay"*. Took **two
  fixes, both invisible to the tests**: the row was unreachable on every SDR,
  and the tap behind it was never filled unless REC was running.
- **The two-window render harness**, and the ⇱ chip guard it unlocked.
- **⇱ chip** — `Order::Middle`, not `Foreground`.
- **#537 false OPEN alerts** — the warm-up clock no longer survives the dial, and
  a spot no longer starts one.
- **SYNOP/SHIP** — BETA, manual §3.3a, quick-start in four languages with PDFs.
  #88 asks Kevin for the off-air check.
- **Branches pruned** — 38 deleted, `.git` 417M → 339M.
- **`sdrbrown.info`** — the site, and the repo homepage pointed at it instead of
  at upstream's releases.
- **Issue form** — the version is now a **required** field. We had a template
  already and it failed three ways: last in the list, optional, and its example
  said `v1.6.0`, *upstream's* scheme.
- **S-meter calibration dropdown removed** — built on a reading from the wrong
  build. Upstream has its own (#642).

### 2026-10-14

- **Search by frequency in SCHEDULE** — `6185`, `6185 kHz` or `6.185`. A search
  is forgiving where the import must not be: with no unit, `153` is longwave *or*
  153 MHz, so both readings are tried and the range rule drops the impossible one.
- **The Bluetooth microphone** — a PipeWire device now appears in Settings →
  Audio by name, with a warning that it takes over the machine's recording.
  Verified on the DUALMIKE.
- **House rules** — "Needs feedback", not "Not proven"; replies are prose.

### 2026-10-13

- **Upstream take, four items, all verified**: the SSTV decoder series (a
  drop-in), FT8 #667 (a merge), CW #625, RTTY #576. Most of upstream's 69 commits
  are features we had already built ourselves.
- **The 2.0.10 changelog entry**, hand-edited from 1,387 lines to 45, and the
  release workflow now **hard-fails notes over 60 lines**.
