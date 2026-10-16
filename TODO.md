# TODO — SDR Oxide Brown

What this fork is working on, kept here so it survives a session and is visible
on the repo. Last updated **2026-10-15**.

Upstream issues are the *maintainer's* queue; items marked `#NNN` are upstream
feature requests **this fork** may pick up, not a promise to upstream.

---

## Now

1. **Undocked mode (#257)** — the SDRuno window set, and the next session's whole
   job. See `SDRUNO-UI-HANDOVER.md` and `UNDOCKED-HANDOVER.md`. The operator's
   call (2026-10-15): the session is saved for this, so nothing else is started
   in it.

2. **The sound device leaving the audio server** — **issue #89**, reported against
   `2.0.13_brown`, documented and **deliberately not next.** Full detail below;
   the short version is that it is probably not a quick fix, and half of it may
   not be needed at all.

   **Report.** Opening the program takes the sound device away from the audio
   server, and closing it does not give it back. The operator, 2026-10-15: *"when I now open the app
   the audio devices selected in the program disappear from my sound settings,
   and I can't see them in cable. That would be ok if they return when I close
   the program but that doesn't happen, I have to unplug or reload pipewire."*

   **The evidence, read from his own configuration:**
   `config.toml` has `audio_output = "Generic KM_B2 Audio, USB Audio
   [Audio · 8087:1024]"` — a USB sound card, selected as the output.

   **The likely cause, and it is nameable: cpal opens ALSA devices directly.**
   cpal's ALSA host hands `snd_pcm_open` the device's own hardware name, which
   **bypasses the sound server and takes the card exclusively**. PipeWire
   therefore loses the node and drops it from the graph — his "disappear from my
   sound settings". When sdroxide exits the PCM closes, but PipeWire does not
   re-acquire the node by itself, so it stays gone until the device is
   replugged or the server restarted.

   **It is the same shape as the Bluetooth work**, and that is the useful part:
   the sound server owns the device, and anything that reaches past it to the
   hardware takes it away from everything else. There we found that
   `pulse:<node>` is accepted by ALSA but cpal cannot open what the host did not
   enumerate — so the fix is not obviously "open it a different way".

   **The intended behaviour, settled by the operator (2026-10-15):** *"for the
   Bluetooth I understand and it was mentioned, because otherwise it would not
   see the device — but for the other devices it's too drastic."*

   So there are **two cases and they are not the same**, and the fix must not
   treat them alike:
   - **A sound-server node** (a Bluetooth headset, a virtual sink) — taking the
     machine's default is the only route that exists, because PipeWire has no
     per-application capture source. It stays, and it stays **warned about**.
   - **An ordinary sound card** — the card must keep working for everything
     else. It is shared through the sound server, and it must **not** disappear
     from the machine's own sound settings while the program runs. Reaching past
     the server to the hardware is the fault here, and there is no reason for it:
     the card is the sound server's to hand out.

   So the shape of the fix is "open the card **through** the sound server, and
   keep the hardware name only as the label the operator picks by" — not "open
   it a different way" in general. Verified against a plain USB card on the bench,
   which is what caught it.

   **The clean-exit question is answered, and it removes the cheap half
   (operator, 2026-10-15):** *"had to test, the card isn't coming back and i have
   to restart pipewire."*

   So it is **not** a kill artefact and **not** a teardown fault. The card is
   closed on exit and PipeWire does **not** reclaim the node by itself — a
   dropped node needs the server restarted, which is its behaviour rather than
   ours. **Which means "release it properly on exit" is not a fix and must not be
   attempted as one**: the only fix is to stop taking the card in the first place.

   That makes this the expensive half and nothing else:

   **What to establish first, before changing anything:**
   - `cat /proc/asound/card*/pcm*/sub*/status` with sdroxide running and stopped —
     does the PCM show `RUNNING` while open, and does it release on exit?
   - Which PCM cpal actually opened: the engine logs the device name it opens.
   - Whether the node returns on a *clean* exit rather than a kill — the operator
     has been `pkill`ing it all evening, and that skips any teardown.
   - If it is the raw-hardware open, the fix is to prefer the `pipewire`/`pulse`
     PCM so the server keeps ownership, and to **warn** when a raw `hw:` device
     is picked — the same treatment the Bluetooth node got, for the same reason.


   this list. See `SDRUNO-UI-HANDOVER.md` and `UNDOCKED-HANDOVER.md`. It wants a
   session of its own rather than the tail of one.
2. **`docs/synop-notes.md`** — keep it current rather than written once. The wrong
   "fixes" it records are what a third reader would otherwise repeat.
3. **`docs/USER_MANUAL.md`** — the ⇱/⇲ chip's `Order::Middle` rule, and the two
   UDP fixes, are worth a line where a reader would look for them. Not urgent;
   the manual is for the operator, not for us.

## From discussion #90 (Kevin, 2026-10-16)

**WEFAX detection at 3855 kHz — "detection, not the signal".** DWD Pinneberg's
**chart** frequency, and four captures of the same transmission minutes apart:

| Time | Result | Width |
|---|---|---|
| 09:45Z | noise, horizontal banding | 1809 |
| 09:47Z | noise above, partial chart below | 1809 |
| 09:57Z | **sheared** — compass rose and `52°N` legible but slanted | **904** |
| 11:15Z | clean, complete chart | 1809 |

The signal is a good fax and the decoder is locking the **wrong line rate or
IOC** — the 904 px half-width and the diagonal shear are the signature. **First
job when the operator is back at the radio (Sunday)**: get the four captures, then
work the decoder's start/rate detection. Care needed — the phasing logic carries
a warning in AGENTS.md (a narrow static stripe is indistinguishable from a
phasing pulse from line data alone; a previous fix regressed #276).

**The rest of #90, checked and answered on the thread:**
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
