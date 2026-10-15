# Weer op de kortegolf — snelstart (sdroxide)

De eenvoudigste dienst op de kortegolfband, en vrijwel niemand luistert er naar.
Schepen en weerstations zenden de **weersituatie waar ze werkelijk zijn** uit als
gecodeerde cijfers op HF RTTY, om de paar uur. Geen internet, geen account, geen
reserveringsslot — alleen een schemaatje op een papiertje.

sdroxide decodes those numbers and shows you the wind, the pressure, the
temperature and the sea state. This card gets you listening in about five minutes.

> **Deze is BETA.** De decoder is getest op vier echte, gepubliceerde berichten
> en verwerkt ze alle vier goed, maar niemand heeft er nog een live mee
> binnengekregen. Klopt er iets niet, of verschijnt er nooit een bericht, laat
> het ons weten — dat is precies wat we willen weten.

*Nederlands: hieronder. Français:
[hfweer-quickstart.fr.md](hfweer-quickstart.fr.md). Italiano:
[hfweer-quickstart.it.md](hfweer-quickstart.it.md). PDF:
[hfweer-quickstart.en.pdf](hfweer-quickstart.en.pdf).*

Vette namen zoals **RTTY** en **WEATHER** zijn knoppen, precies zoals ze op het
scherm staan. `Settings > Radio` is een menupad.

---

## What you need

- **Elke ontvanger die sdroxide ondersteunt**, met een antenne die HF haalt.
- **Verder niets.** U luistert, u zendt niet, en een vergunning is niet nodig.

Een draadantenne van een paar meter voldoet. Hebt u al een HF-opstelling voor
FT8 of RTTY, dan gebruikt u dezelfde antenne en dezelfde ontvanger.

---

## De drie frequenties

Stem een van deze af. Ze dragen hetzelfde materiaal, met verschillende vermogens
en voor verschillende delen van de wereld.

| kHz | Opmerkingen |
|---|---|
| **4583** | Weerbericht van de Duitse dienst, 24 uur |
| **7646** | hetzelfde schema, wereldwijd |
| **10100.8** | hetzelfde, hoger vermogen — het beste met een kleine antenne |

Begin met **7646** als u het niet zeker weet.

---

## Het schema

Er gaan twee verschillende codes uit, ongeveer elke zes uur:

| UTC | Code | Van |
|---|---|---|
| 00:35, 12:10 | **SYNOP** | landweerstations |
| 02:00, 07:35 | **SHIP** | schepen op zee |
| 08:15, 20:15 | **BOEI** | boeien |

Vermenigvuldig met twee voor de zomertijd in uw eigen land, en onthoud dat de
'tijden UTC zijn, niet lokaal.

**Bepaal vooraf wat er aan komt.** Kies een uur of zo voor een van deze tijden en
begin enkele minuten eerder te luisteren — een schema dat zo gaat beginnen is
het hele verschil tussen iets horen en niets horen.

**Dit zijn de stilste uren op de band.** Niemand roept iemand aan. Wie gezelschap
wil, luistert om 02:00 UTC en geniet van het ontbreken ervan.

---

## Opzetten

1. Kies **RTTY** uit het DIGITAL-menu in de band/mode-venster.
2. Stel **50 baud** en **170 Hz shift** in. Deze diensten gebruiken 50 baud — niet
   de 45,45 die amateur-RTTY vaak gebruikt, en niet 75.
3. Stem af op **7646 kHz**. Het signaal is smal; 2–4 kHz breed in de waterfall
   is ruim voldoende.
4. Zet de ontvanger op **USB**. Dit zijn enkelzijbandtransmissies.
5. Laat het rustig.

Hoort u helemaal niets, dan is de shift of de zijband meestal de oorzaak, niet
de frequentie.

---

## Wanneer een bericht binnenkomt

De regel ziet er zo uit en betekent op zichzelf niets:

```
BBXX BRAVO 20123 99252 10595 41494 81412 10285 20269 40100 53012 79586 8597/ …
```

Naast het berichtenveld verschijnt een **WEATHER**-knop met het aantal gehoorde
berichten. Druk erop.

Elk bericht geeft het **station of schip**, het **tijdstip van waarneming**, de
**positie**, en daarna het weer: windrichting en -snelheid **in de eenheid die het
bericht zelf opgeeft**, lucht- en dauwpuntstemperatuur, luchtdruk, bewolking,
zicht, weersverschijnselen, en — bij een schip — koers, snelheid, golfhoogte en
deining.

**De oorspronkelijke regel staat boven de uitleg.** Dat is bewust. Wilt u een
getal controleren, dan ziet u de cijfers waar het vandaan komt — en gelooft u
het niet, dan heeft u gelijk dat te zeggen.

**Een streepje is een antwoord.** Groepen als `8597/` betekenen *niet gerapporteerd* —
wat niet nul is en ook niet ontbreekt. Ze worden getoond als **niet gerapporteerd**
in plaats van leeg, omdat leeg onze eigen fout zou doen lijken.

Al wat de decoder niet herkent, wordt bewaard en getoond in de eigen woorden van
het station. Wie zijn berichten met de hand typt, zet het interessantste deel er
precies zo in, en dat is het deel dat de moeite waard is.

Berichten worden alleen tijdens deze sessie bewaard. De lijst begint morgen leeg —
een bericht dat u morgen leest, zou een leugen zijn over het waarnemingstijdstip.

---

## Wat dit niet is

**Het is geen verwachting.** Het is een waarneming, van één schip, op één moment.
Bent u aan het varen plannen, dan is dit één gegeven naast vele andere, en niet
het gegeven waarop u beslissingen baseert.

**Het vervangt de officiële dienst niet.** De mensen die deze uitzendingen doen,
doen dat om veiligheidsredenen. Het bericht lezen is het leuke deel.

---

## Als er niets gebeurt

| Wat u ziet | Wat u kunt proberen |
|---|---|
| Helemaal geen geluid | Controleer frequentie en mode |
| Geluid, maar geen tekens | De shift is waarschijnlijk 170 en niet 425/450 — hier is het **170** |
| Tekens, maar onzin | Meestal de **zijband** — probeer USB |
| Tekens, geen WEATHER-knop | De regel was geen gecodeerd bericht. Weerberichten en gewone RTTY delen de band. |
| WEATHER-knop, verkeerde getallen | **Meld het ons.** Dat is precies de fout die we zoeken. |

De laatste regel helpt het meest. Eén verkeerd getal is meer waard dan twintig
juiste, en wij kunnen het niet vinden zonder iemand die meeluistert.

---

## Meer lezen

- [`USER_MANUAL.md` §3.3a](USER_MANUAL.md) — de volledige referentie voor deze decoder.
- `docs/synop-notes.md` in de broncode — waar het format vandaan komt, en welke
  delen nog onzeker zijn.