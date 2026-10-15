# Weather on shortwave — quick-start (sdroxide)

The plainest service on the shortwave band, and almost nobody listens to it.
Ships and weather stations broadcast the **weather where they actually are** as
coded numbers on HF RTTY, every few hours. No internet, no account, no timeslot
to book — just a schedule you can read off a piece of paper.

sdroxide decodes those numbers and shows you the wind, the pressure, the
temperature and the sea state. This card gets you listening in about five minutes.

> **This one is BETA.** The decoder is tested against four real published
> messages and decodes all of them correctly, but nobody has yet read one off the
> air with it. If you try it and the numbers look wrong, or a report never
> appears, please tell us — that is exactly what we want to know.

*Nederlands: [hfweer-quickstart.nl.md](hfweer-quickstart.nl.md). Français:
[hfweer-quickstart.fr.md](hfweer-quickstart.fr.md). Italiano:
[hfweer-quickstart.it.md](hfweer-quickstart.it.md). PDF:
[hfweer-quickstart.en.pdf](hfweer-quickstart.en.pdf).*

Bold names such as **RTTY** and **WEATHER** are buttons exactly as they appear
on screen. `Settings > Radio` is a menu path.

---

## What you need

- **Any receiver sdroxide supports**, and an antenna that reaches HF.
- **Nothing else.** You are listening, not transmitting, and no licence is
  involved.

A wire antenna a few metres long works. If you already have an HF setup for
FT8 or RTTY, this uses the same antenna and the same receiver.

---

## The three frequencies

Tune one of these. They carry the same material at different powers and for
different parts of the world.

| kHz | Notes |
|---|---|
| **4583** | German weather service broadcast, 24 hours |
| **7646** | the same schedule, worldwide |
| **10100.8** | the same, higher power — best on a small antenna |

Start with **7646** if you are not sure.

---

## The schedule

Two different codes go out, roughly every six hours:

| UTC | Code | From |
|---|---|---|
| 00:35, 12:10 | **SYNOP** | land weather stations |
| 02:00, 07:35 | **SHIP** | ships at sea |
| 08:15, 20:15 | **BUOY** | buoys |

Multiply by two for the summer time in your own country, and remember that the
times are UTC, not local.

**Work out what is next before you sit down.** Pick a time an hour or so before
one of these, and start listening a few minutes early — a schedule that is about
to start is the whole difference between hearing one and hearing nothing.

**These are among the quietest hours on the band.** Nobody is calling anybody.
If you want company, listen at 02:00 UTC and enjoy that there is none.

---

## Setting up

1. Choose **RTTY** from the DIGITAL menu of the band/mode popup.
2. Set **50 baud** and **170 Hz shift**. These services use 50 baud — not the
   45.45 that amateur RTTY commonly uses, and not 75.
3. Tune to **7646 kHz**. The signal is narrow; panadapter at 2–4 kHz wide is
   plenty.
4. Switch the receiver to **USB**. These are single-sideband transmissions.
5. Leave it alone.

If you hear nothing at all, the most likely reason is the shift or the
sideband, not the frequency.

---

## When a report arrives

The line looks like this and means nothing on its own:

```
BBXX BRAVO 20123 99252 10595 41494 81412 10285 20269 40100 53012 79586 8597/ …
```

A **WEATHER** chip appears beside the message box with the number of reports
heard. Press it.

Each report gives the **station or ship**, the **time it was observed**, its
**position**, and then the weather: wind direction and speed **in the unit the
report asked for**, air and dew-point temperature, pressure, cloud, visibility,
weather, and — for a ship — course, speed, waves and swell.

**The raw line is kept above the decoding.** That is deliberate. If you want to
check a number, you can see the digits it came from, and if you do not believe
it, you are right to say so.

**A slash is an answer.** Groups printed like `8597/` mean *not reported* —
which is neither zero nor missing. They show as **not reported** rather than
blank, because a blank reads as our fault rather than the station's.

Anything the decoder does not recognise is kept and shown in the station's own
words. People who type their reports by hand put the most interesting part in
exactly that way, and it is the part worth reading.

Reports are kept for the session only. Tomorrow's list starts empty — a report
read tomorrow would be a lie about when it was observed.

---

## What this is not

**It is not a forecast.** It is an observation, from one ship, at one moment. If
you are planning a passage, this is one input among many and not the one to make
decisions from.

**It is not a substitute for the official service.** The people who run these
broadcasts do it for safety. Reading the report is the fun part.

---

## If nothing happens

| What you see | What to try |
|---|---|
| No audio at all | Check the frequency, and the mode |
| Audio but no characters | The shift is probably 170 and not 425/450 — these use **170** |
| Characters but nonsense | Usually the **sideband** — try USB |
| Characters, no WEATHER chip | The line was not a coded report. Weather traffic and ordinary RTTY share the band. |
| WEATHER chip, wrong numbers | **Please tell us.** That is the bug this is looking for. |

The last row is the one that helps most. A wrong number is worth more than
twenty right ones, and there is no way for us to find it without someone
listening.

---

## Where to read more

- [`USER_MANUAL.md` §3.3a](USER_MANUAL.md) — the full reference for this decoder.
- `docs/synop-notes.md` in the source repository — where the format came from,
  and which parts are still uncertain.