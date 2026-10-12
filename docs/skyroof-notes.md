# SkyRoof — notes

Notes from **SkyRoof** by Alex Shovkoplyas, **VE3NEA** (the CW Skimmer / SkySDR
author): <https://github.com/VE3NEA/SkyRoof>, site
<https://ve3nea.github.io/SkyRoof>. A **Windows (C#/WPF) satellite + SDR app**
for "Hams and satellite enthusiasts", **AGPL-3.0**. Operator pointed at it,
2026-10-12.

**Licence: read it for ideas, do not copy code.** It is AGPL-3.0; this fork is
GPL-3.0-or-later, and lifting AGPL source would change our obligations.

## What it does

Satellite tracking + pass prediction (Sky View, Earth View, Time Line, Pass
List); an SDR waterfall covering the **whole satellite segments** on VHF/UHF
with **satellite names and Doppler-corrected transponder boundaries on the
frequency scale**; SSB/CW/FM RX with RIT and **Doppler tracking**; audio **and
I/Q out via VAC or UDP**; telemetry decoders (**FSK/AFSK/GFSK/GMSK/BPSK**) with
**automatic SatNOGS submission**; **SSTV** (Robot/PD), **SSDV** image
reconstruction (combining several passes), **Codec2** voice playback; an **FT4
console** for linear-transponder satellites; QSO logging + a **pass scheduler**;
CAT control; rotator control.

## Cross-check against our own satellite code

We already do a lot of it — checked before treating any of this as new work:

- **Pass prediction — covered** (`#623`). `sdroxide-solar/src/satellites.rs`'s
  `Satellite::next_passes`, and the SAT window keeps a pass memo and draws the
  pass curves (`sat.rs`: `PassCurve`, `SatPass`, the `pass` memo).
- **Doppler + transponder uplink — covered, largely** (`#651`).
  `sdroxide-types/src/satlock.rs` has `doppler_rx_hz` / `doppler_tx_hz`, the
  `SatUplink` transponder mapping (inverting and non-inverting), and the SAT
  window shows DOPPLER RX/TX and DOWNLINK/UPLINK. The lock tunes **VFO A** and
  the engine derives the uplink; the rig's **own split (VFO A = downlink, VFO B
  = uplink)** is the one nuance that may not be done.
- Also already ours: transponder frequencies (`satfreq.rs`), SAT lock config
  (`satcfg.rs`), rotator control, and a QO-100 quick-start.

## SkyRoof-only ideas worth considering (fit this fork)

- **Doppler-corrected panadapter** — satellite names and transponder-segment
  boundaries drawn on the frequency scale, so the whole passband is labelled and
  the segments follow Doppler. Our panadapter corrects the DDC but does not
  label the picture. A visual feature, no wire change.
- **Satellite telemetry decoders** (FSK/AFSK/GFSK/GMSK/BPSK) with SatNOGS
  submission — fits our "decoding" identity; a new wideband/audio lane like the
  existing ones.
- Further ideas (not scoped): **SSDV** image rebuild across passes, **Codec2**
  voice playback, the **FT4 transponder console**, a **pass scheduler**, and
  **I/Q output over UDP** (which is our **#594**).
