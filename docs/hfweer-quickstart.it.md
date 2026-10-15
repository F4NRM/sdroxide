# Il meteo sulle onde corte — avvio rapido (sdroxide)

Il servizio più semplice della banda onde corte, e quasi nessuno lo ascolta.
Navi e stazioni meteo trasmettono **il tempo dove si trovano davvero** come cifre
codificate in RTTY HF, ogni poche ore. Niente internet, niente account, niente
fasce orarie da prenotare — solo un orario scritto su un foglio di carta.

sdroxide decodes those numbers and shows you the wind, the pressure, the
temperature and the sea state. This card gets you listening in about five minutes.

> **Questo è in versione BETA.** Il decodificatore è stato provato su quattro
> messaggi reali pubblicati e li decodifica tutti correttamente, ma nessuno ha
> ancora ricevuto un bollettino in diretta. Se i numeri sembrano sbagliati, o se
> un bollettino non compare mai, fatecelo sapere: è esattamente quello che vogliamo
> sapere.

*Nederlands: [hfweer-quickstart.nl.md](hfweer-quickstart.nl.md). Français:
[hfweer-quickstart.fr.md](hfweer-quickstart.fr.md). Italiano: qui sopra.
PDF:
[hfweer-quickstart.en.pdf](hfweer-quickstart.en.pdf).*

I nomi in grassetto come **RTTY** e **WEATHER** sono pulsanti, esattamente come compaiono
sullo schermo. `Settings > Radio` è un percorso di menu.

---

## What you need

- **Qualsiasi ricevitore supportato da sdroxide** e un'antenna che raggiunga le HF.
- **Nient'altro.** Si ascolta, non si trasmette, e non serve alcuna licenza.

Basta un'antenna a filo di pochi metri. Se avete già un impianto HF per FT8 o RTTY,
userà la stessa antenna e lo stesso ricevitore.

---

## Le tre frequenze

Sintonizzate una di queste. Trasmettono lo stesso materiale, con potenze diverse e
per parti diverse del mondo.

| kHz | Note |
|---|---|
| **4583** | Diffusione del servizio meteorologico tedesco, 24 ore |
| **7646** | lo stesso orario, mondiale |
| **10100.8** | lo stesso, potenza maggiore — ideale con una piccola antenna |

Iniziate da **7646** se non siete sicuri.

---

## L'orario

Vengono trasmessi due codici diversi, circa ogni sei ore:

| UTC | Codice | Da |
|---|---|---|
| 00:35, 12:10 | **SYNOP** | stazioni meteorologiche terrestri |
| 02:00, 07:35 | **SHIP** | navi in mare |
| 08:15, 20:15 | **BOA** | boe |

Raddoppiate per l'ora legale del vostro paese, e ricordate che gli orari sono in UTC,
non locali.

**Determinate prima quale sarà la prossima trasmissione.** Scegliete un'ora circa prima
di una di queste e cominciate ad ascoltare qualche minuto prima: un orario sta per
iniziare è tutta la differenza fra sentire un bollettino e non sentirne nessuno.

**Sono fra le ore più silenziose della banda.** Nessuno chiama nessuno. Se volete
compagnia, ascoltate alle 02:00 UTC e godete l'assenza.

---

## Configurazione

1. Scegliete **RTTY** dal menu DIGITALE nella finestra banda/modalità.
2. Impostate **50 baud** e **170 Hz di shift**. Questi servizi usano 50 baud — non i
   45,45 tipici della RTTY amatoriale, né 75.
3. Sintonizzate **7646 kHz**. Il segnale è stretto: 2–4 kHz nel waterfall bastano.
4. Portate il ricevitore in **USB**. Sono trasmissioni a banda laterale unica.
5. Lasciatelo così.

Se non sentite nulla, la causa più probabile è lo shift o la banda laterale, non la
frequenza.

---

## Quando arriva un bollettino

La riga ha questo aspetto e da sola non significa nulla:

```
BBXX BRAVO 20123 99252 10595 41494 81412 10285 20269 40100 53012 79586 8597/ …
```

Accanto al riquadro dei messaggi compare il pulsante **WEATHER** con il numero di
bollettini ricevuti. Premetelo.

Ogni bollettino dà la **stazione o la nave**, l'**ora dell'osservazione**, la
**posizione**, e poi il tempo: direzione e velocità del vento **nell'unità che il
bollettino stesso indica**, temperatura dell'aria e del punto di rugiada,
pressione, nuvolosità, visibilità, tempo in atto e — per una nave — rotta,
velocità, mare e onde di fondo.

**La riga originale è conservata sopra la decodifica.** È una scelta. Se volete
verificare un numero vedete le cifre da cui proviene; e se non ci credete, avete
ragione a dirlo.

**La barra è una risposta.** Gruppi come `8597/` significano *non trasmesso* — che non
è zero né un'assenza. Vengono mostrati come **non trasmesso** e non come vuoto,
perché un vuoto farebbe pensare a un nostro errore.

Tutto ciò che il decodificatore non riconosce viene conservato e mostrato con le
parole della stazione. Chi scrive a mano i propri bollettini mette la parte più
interessante proprio così, ed è quella che vale la pena di leggere.

I bollettini sono conservati solo per la sessione. Domani l'elenco riparte vuoto: un
bollettino letto domani mentirebbe sull'ora dell'osservazione.

---

## Che cosa non è

**Non è una previsione.** È un'osservazione, di una sola nave, in un solo istante. Per
pianificare una traversata è uno fra molti dati, e non quello su cui decidere.

**Non sostituisce il servizio ufficiale.** Chi gestisce queste trasmissioni lo fa per
sicurezza. Leggere il bollettino è la parte divertente.

---

## Se non succede nulla

| Cosa vedete | Cosa potete provare |
|---|---|
| Nessun suono | Controllate frequenza e modo |
| Suono ma nessun carattere | Lo shift è probabilmente 170 e non 425/450 — qui è **170** |
| Caratteri incomprensibili | Di solito la **banda laterale** — provate USB |
| Caratteri, nessun pulsante WEATHER | La riga non era un bollettino codificato. Meteo e RTTY ordinario condividono la banda. |
| Pulsante WEATHER, numeri sbagliati | **Fatecelo sapere.** È il difetto che stiamo cercando. |

L'ultima riga è la più utile. Un numero sbagliato vale più di venti giusti, e non
possiamo scoprirlo senza qualcuno che ascolta.

---

## Per saperne di più

- [`USER_MANUAL.md` §3.3a](USER_MANUAL.md) — il riferimento completo di questo decodificatore.
- `docs/synop-notes.md` nel sorgente — da dove viene il formato e quali parti sono
  ancora incerte.