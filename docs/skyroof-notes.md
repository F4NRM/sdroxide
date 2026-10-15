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

- **Pass prediction — covered** (`#623`), and the **pass scheduler is BUILT**
  (`4dee2184`, the PASSES tab). `sdroxide-solar/src/satellites.rs`'s
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

- ~~**Doppler-corrected panadapter**~~ — **BUILT 2026-10-14** (`fac88c05`).
  `sdroxide_types::satcfg::sat_bands` shifts each **downlink** by the correction
  in force and `spectrum_view::draw_sat_bands` draws them on the scale, labelled
  where there is room. Four tests pin the arithmetic, including the sign in both
  directions — drawing at the *published* frequency puts the label exactly as far
  from the signal as the correction, which is the fault the feature exists to
  prevent. Only downlinks: an uplink is on another band entirely. The operator's
  own frequency table wins over the built-in one, as the SAT window does.
- **Satellite telemetry decoders** (FSK/AFSK/GFSK/GMSK/BPSK) with SatNOGS
  submission — **being worked, AFSK first** (the operator's call: submission
  first would have nothing to submit). What is already established, measured
  rather than assumed:

  - **BPSK is ours** — `sdroxide-qo100` decodes the QO-100 BPSK-400 beacon and
    the AO-40 uncoded frames off raw IQ. This note was stale on that half.
  - **SatNOGS submission does not exist.** The only "SatNOGS" in the tree is a
    comment about EasyComm rotator protocols.
  - **Both halves of the AFSK path are already written.** `sdroxide-dsp::afsk`
    is the Bell 202 demodulator (1200/2200 Hz at 1200 baud for VHF, 1600/1800 at
    300 for HF) and `sdroxide-ax25::hdlc` is the bit level — flags, zero-bit
    stuffing, NRZI and the FCS. ISS telemetry is AX.25-framed, so the bit layer
    is not work.
  - **The first mission is the ISS**, 145.827 MHz FM, 1200 baud **AFSK-ASCII**,
    in the AMSAT **AOS**-style format of 16-byte fields that ISS and a good
    number of cubesats share. Funcube-1 (AO-73) is the counter-example worth
    noting: its telemetry is **BPSK** at 145.950 MHz, so it belongs with the
    QO-100 work rather than here.
  - **"AOS" means two different things** and the name must not be used without
    saying which: the amateur telemetry format above, and **CCSDS AOS Space Data
    Link Protocol** (732.0-B-4), an unrelated high-rate spacecraft protocol. A
    module called `aos` would be ambiguous to the next reader.
  - **Not started, deliberately:** the per-mission field layouts. Those are
    published per mission and change, so they want writing out from a published
    table with real frames to test against — not from memory.
- Further ideas (not scoped): **SSDV** image rebuild across passes, **Codec2**
  voice playback, the **FT4 transponder console**, a **pass scheduler**, and
  **I/Q output over UDP** (which is our **#594**).
