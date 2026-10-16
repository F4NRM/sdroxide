# TODO — SDR Oxide Brown

What this fork is working on, kept here so it survives a session and is visible
on the repo. Last updated **2026-10-15**.

Upstream issues are the *maintainer's* queue; items marked `#NNN` are upstream
feature requests **this fork** may pick up, not a promise to upstream.

---

## Now

1. **Undocked mode (#257)** — the SDRuno window set, and the next session's whole
   job. See `SDRUNO-UI-HANDOVER.md` and `UNDOCKED-HANDOVER.md`. The operator's
   call: the session is saved for this, so nothing else is started in it.

2. **WEFAX detection** — the one real bug from discussion #90. DWD Pinneberg at
   3855 kHz, four captures of the same transmission minutes apart, and the decoder
   locking the **wrong line rate or IOC**: the 09:57 picture has a compass rose
   reading *SEESCHIFFAHRT DWD* and two `52°N` labels — a perfectly good fax — but
   it is sheared and **904 px wide against 1809**. Full detail in the #90 section
   below. **First job when the operator is back at the radio (Sunday)**; it needs
   his captures, and the phasing logic it touches carries a warning (a previous
   fix regressed #276).

3. **`docs/synop-notes.md`** — keep it current rather than written once. The wrong
   "fixes" it records are what a third reader would otherwise repeat.

4. **`docs/USER_MANUAL.md`** — the ⇱/⇲ chip's `Order::Middle` rule, and the two
   UDP fixes, are worth a line where a reader would look for them. Not urgent;
   the manual is for the operator, not for us.

## Sound device leaving the audio server (#89) — FIXED, awaiting his confirmation

Reported against `2.0.13`: opening the program took the sound device out of the
audio server, and closing it did not give it back.

**The cause was ours, and it was not how the card is opened.** Two theories died
by measurement — that cpal's hardware-name open takes the card (`hw:`,
`sysdefault:`, `pulse:` and `default` each tested; none did) and that our
`wpctl set-default` did it (tested; it did not). WirePlumber's own journal named
it instead:

    s-monitors: Failed to create ALSA node …: Object activation aborted:
      PipeWire proxy destroyed

**The client that vanished was ours.** The Settings → Audio page asked the
patchbay what it held **on every frame it drew**, spawning a `pactl` process each
time — tens a second, every one a PipeWire client connecting and being destroyed,
and one landing inside WirePlumber's activation window aborted it. Fixed in
`0d1dd7a1`: a pure shape check answers the common case with no process at all,
and the list is cached for when it is genuinely needed.

**Two things still stand from the report:**

- **The intended behaviour, settled by the operator:** taking the machine's
  default is right for a **sound-server node** (a Bluetooth headset — the only
  route that exists, because PipeWire has no per-application capture source) and
  **too drastic for an ordinary sound card**, which must keep working for
  everything else. That distinction is in the settings warning already.
- **A clean exit does not return the card either** — PipeWire does not reclaim a
  dropped node by itself. That is why "release it properly on exit" was never the
  fix, and why the cause had to be found instead.

**Awaiting the operator's word that the card now stays put.**

## From discussion #90 (Kevin, 2026-10-16)

**WEFAX detection** — see *Now* above for the evidence; it is the one real bug
here and it is first for Sunday.

**The rest, checked and answered on the thread:**
- *Settings rows overlapping* — **not reproducible**, here or on his machine with
  the trace. Every row at its own y. Left open in case it returns.
- *Sound 3 s behind the waterfall* — every queue is bounded (server lane 8 blocks,
  worklet 250 ms, desktop ring ~0.5 s) and the hidden-tab case is fixed by
  `fresh_tail`. **Asked which client and which output device** — 3 s smells like a
  Bluetooth/USB buffer, not a queue.
- *FT8 scroll bar out of sync* — the list never had stick-to-bottom, so nothing was
  removed. **Hypothesis: streaming decodes grow the list under a scroll position
  set for a finished slot.** Asked him to confirm the drift.
- *"Many parameters not persistent"* — his own tests pass, the config parses, no
  quarantine. **Asked which settings** — chaseable without a radio.
- *s-meter calibration deleted* — not a bug; removed on his word (wrong-build
  reading). Upstream has its own at #642.

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
