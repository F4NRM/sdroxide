# A Bluetooth microphone — where this stands

**Status: the Bluetooth half is solved. It is blocked one layer below it, on
device enumeration, and that is a real piece of work rather than a config
mistake.**

Fun-project framing, 2026-10-13: the operator has a **Midland Dual Mike
Wireless**, a Bluetooth CB microphone that only works with Midland's own phone
app. *"It's useless everywhere else since there is no good BT connection for
general CBs.. but.. It's a BT device.. do you think we could make it work with our
program?"* **Voice only, and a gimmick** — no digital modes through it, no new
mode, no wire change.

## What the device is

Researched from Midland's own pages and the published quick guide, then confirmed
against the hardware. The important find is in their manual: there is a section
headed **"PAIRING WITH OTHER BLUETOOTH DEVICES"** — hold **secondary PTT and
VOL+** together until the LEDs alternate. **It is not locked to the app.** (Phone
pairing is a *different* combo — *PTT CBTalk + VOL+* — and using the wrong one is
a good way to conclude it will not pair.)

Confirmed on the bench:

| | |
|---|---|
| Name | `DUALMIKE`, `54:7A:52:00:3B:17`, public address |
| Class | `0x00240414`, `Icon: audio-card` |
| Profiles | **Handsfree (0x111e)**, Headset (0x1108), Headset HS (0x1131), Audio Sink (0x110b), A2DP (0x110d), Serial Port (0x1101) |
| State | paired, bonded, **connected** |

**`Handsfree` is the one that matters** — that is the profile with a microphone in
it. A2DP alone is output-only, which would have been the dealbreaker; HFP makes it
a real two-way headset.

**`TY` (`D8:1F:12:63:F3:6D`) is a second, separate device** — BLE-only, custom
service `0xa201`, manufacturer data, no `Class:`, no audio profile. That is the
**PTT-Dual**, Midland's steering-wheel button, described in their copy as a
*"Bluetooth Low Energy (B.L.E.) button to talk hands-free."* A bonus, not a
distraction: it is a wireless button, and a wireless button is what a PTT is.

## The host side is ready

Nothing to install. This machine has **PipeWire 1.6.9 + WirePlumber 0.5.18**,
which carry their **own HFP backend** — `module-bluez5-device.c`, **no `ofono`
needed** — and `libspa-codec-bluez5-g722.so` is present, so it can do *wideband*
hands-free rather than the usual 8 kHz.

Both endpoints are live while it is connected:

```
bluez_input.54:7A:52:00:3B:17        # the microphone
bluez_output.54_7A_52_00_3B_17.1     # the 3 W speaker
```

## The blocker: we enumerate one layer below the devices

**sdroxide asks ALSA for its devices** — `cpal::default_host()`, which on Linux is
ALSA. And ALSA cannot see PipeWire's graph. It exposes a *bridge* and nothing
behind it:

```
$ aplay -L
null lavrate samplerate speexrate jack oss
pipewire  pulse  speex upmix vdownmix
default   hdmi:CARD=…  sysdefault:CARD=…  front:CARD=…
```

So the Bluetooth devices **will never appear in the settings list by name**,
however long anyone looks — and that is exactly the report: *"in the settings i
cannot find the bluez options"*, while the patchbay (*cable*) shows the speaker
wired up, because that is the PipeWire layer. Both observations are correct;
they are different floors of the same building.

The relevant code is `crates/sdroxide-audio/src/lib.rs`:
`enumerate_devices(host, output)` → `cpal::default_host()`, and the settings row is
**General → "Your audio (speakers / microphone)"** (`settings/general.rs`), whose
pick goes through `set_audio_device(output, name)`.

## Route A — today, no code, heavy-handed

Make the mike PipeWire's **default** and go through the bridge:

```sh
wpctl status                 # find the DUALMIKE source and sink node ids
wpctl set-default <sink-id>  # then again for the source
```

Then pick **`pipewire`** (or `pulse`) in sdroxide for both **Output** and
**Input**.

**The cost is the reason this is Route A and not the answer:** it hands the mike
the system default for *everything*, so your desktop audio moves onto a CB mike to
make one program work. For a gimmick it is fine. For a feature it is not.

## Route B — the real fix, and what it buys

**A PipeWire-aware device list**, so nodes appear by name beside the ALSA cards and
can be picked independently.

What it buys, and this is why it is worth more than the mike:

- the **Bluetooth mike by name** — no system default handed over;
- **virtual sinks and loopbacks** — a null sink, a mix, a recording tap;
- **any other PipeWire node** a listener has arranged in a patchbay;
- and it removes the confusing gap where the program and the patchbay disagree
  about what devices exist.

**The shape, unproven and deliberately not estimated:**

1. **cpal will not do it.** Its Linux host is ALSA; there is no PipeWire backend to
   switch on. So this is a **second audio host**, or a PipeWire-native path beside
   cpal — `pipewire-rs` (the `pipewire` crate) binds libpipewire, and both are
   already on this machine because PipeWire ships them.
2. **What has to change** is the enumeration seam and the two opens — this crate
   already funnels devices through `enumerate_devices` / `device_names` and a
   small picker, so the blast radius is the audio crate and the settings row, not
   the engine.
3. **The trap to design around:** a PipeWire stream is a *graph node*, not a device
   you open and close. sdroxide currently opens a cpal stream and falls back
   across several configs on failure (`pick_stream_config`); a PipeWire path needs
   the same fallback story written against a different API, or the first unhappy
   format will look like "the mike doesn't work".
4. **Do not start it for the mike alone.** Route A costs ten minutes and proves
   whether the *voice path* is even usable; if the answer is no, Route B fixes
   enumeration and the feature still does not exist.

## What is NOT verified, and must not be claimed

- ~~**Whether sdroxide's "Your audio → Input" is used for voice transmit at
  all.**~~ **ANSWERED 2026-10-15: yes, it is.** The engine holds one `mic`
  handle, drains it into `mic_fifo`, and `fill_tx_audio_fifo_depth` is what feeds
  the transmit block — so the chosen input is the voice transmit source and this
  project has a destination. It also already notices when it goes out silent:
  `report_silent_microphone` raises a notice naming whether the input sent
  nothing, there was no microphone open, or the TCI client sent none.

  **One thing came out of checking it.** That notice pointed at **Settings →
  General**, which is where the microphone used to be: the sound devices moved to
  **Settings → Audio**, and the message went on naming the old page. A message
  that sends the operator to the wrong room to fix a wrong setting is this fork's
  own bug class, and it was introduced by the Audio page the same day. Fixed, and
  the wording now lives in one free function (`silent_mic_notice`) so a test can
  call it rather than assembling a copy beside it.
- **HFP round-trip quality and latency** through the mike: unmeasured. Expected
  8 kHz CVSD or 16 kHz mSBC, 50–150 ms. Fine for CB voice; irrelevant for digital
  modes, which would keep using the radio's own audio.
- **Whether A2DP can coexist** with HFP on this device. Normally the link takes
  one at a time; if only HFP is active, music through it is not a thing.
- **Route B entirely.** Not scoped, not estimated, nothing written.

## Cheapest next step

**Not more Bluetooth.** Prove the voice path on *any* microphone — the operator
speaks into the ordinary sound card input and sees whether it reaches the
transmitter. That is ten minutes, needs no radio to *receive*, and tells us whether
this project has a destination at all. If it does, Route A makes the mike work
today and Route B becomes a decision about the audio layer rather than a favour to
a microphone.
