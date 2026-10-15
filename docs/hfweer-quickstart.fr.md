# La météo sur les ondes courtes — démarrage rapide (sdroxide)

Le service le plus simple de la bande ondes courtes, et presque personne ne l'écoute.
Navires et stations météorologiques diffusent **la météo là où ils se trouvent
réellement**, sous forme de chiffres codés en RTTY HF, toutes les quelques heures.
Pas d'internet, pas de compte, pas de créneau à réserver — seulement un horaire
tenu sur un bout de papier.

sdroxide decodes those numbers and shows you the wind, the pressure, the
temperature and the sea state. This card gets you listening in about five minutes.

> **Celui-ci est en version BÊTA.** Le décodeur a été testé sur quatre vrais messages
> publiés et les décode correctement, mais personne n'a encore reçu de rapport en
> direct. Si les chiffres vous semblent faux, ou si aucun rapport n'apparaît,
> dites-le nous — c'est exactement ce que nous voulons savoir.

*Nederlands: [hfweer-quickstart.nl.md](hfweer-quickstart.nl.md). Français: ci-dessus.
Italiano:
[hfweer-quickstart.it.md](hfweer-quickstart.it.md). PDF:
[hfweer-quickstart.en.pdf](hfweer-quickstart.en.pdf).*

Les noms en gras comme **RTTY** et **WEATHER** sont des boutons tels qu'ils apparaissent
à l'écran. `Settings > Radio` est un chemin de menu.

---

## What you need

- **N'importe quel récepteur compatible sdroxide**, et une antenne qui couvre les
> HF.
- **Rien d'autre.** Vous écoutez, vous n'émettez pas, et aucune licence n'est requise.

Une antenne filaire de quelques mètres suffit. Si vous avez déjà une installation
HF pour FT8 ou RTTY, elle utilise la même antenne et le même récepteur.

---

## Les trois fréquences

Réglez l'une d'elles. Elles transportent le même contenu, avec des puissances
différentes et pour différentes parties du monde.

| kHz | Remarques |
|---|---|
| **4583** | Diffusion du service météo allemand, 24 h |
| **7646** | le même horaire, mondial |
| **10100.8** | le même, puissance supérieure — idéal avec une petite antenne |

Commencez par **7646** en cas de doute.

---

## L'horaire

Deux codes différents sont émis, environ toutes les six heures :

| UTC | Code | De |
|---|---|---|
| 00:35, 12:10 | **SYNOP** | stations météorologiques terrestres |
| 02:00, 07:35 | **SHIP** | navires en mer |
| 08:15, 20:15 | **BOUE** | bouées |

Multipliez par deux pour l'heure d'été de votre pays, et rappelez-vous que ces
heures sont en UTC, pas locales.

**Déterminez la prochaine émission avant de vous installer.** Choisissez une heure
un peu avant l'une de celles-ci et commencez à écouter quelques minutes plus tôt —
un horaire sur le point de commencer, c'est toute la différence entre entendre un
rapport et n'entendre rien.

**Ce sont parmi les heures les plus calmes de la bande.** Personne n'appelle
personne. Si vous cherchez de la compagnie, écoutez à 02:00 UTC et profitez de
cette absence.

---

## Réglage

1. Choisissez **RTTY** dans le menu NUMÉRIQUE de la fenêtre bande/mode.
2. Réglez **50 bauds** et **170 Hz de shift**. Ces services utilisent 50 bauds —
   pas les 45,45 de l'amateur radio, ni 75.
3. Réglez sur **7646 kHz**. Le signal est étroit ; 2 à 4 kHz suffisent dans le
   waterfall.
4. Passez le récepteur en **USB**. Ce sont des émissions en bande latérale unique.
5. Laissez-le tranquille.

Si vous n'entendez rien du tout, la cause la plus probable est le shift ou la bande
latérale, pas la fréquence.

---

## Quand un rapport arrive

La ligne ressemble à ceci et ne signifie rien à elle seule :

```
BBXX BRAVO 20123 99252 10595 41494 81412 10285 20269 40100 53012 79586 8597/ …
```

Un bouton **WEATHER** apparaît à côté de la zone de message avec le nombre de
rapports reçus. Appuyez dessus.

Chaque rapport donne la **station ou le navire**, l'**heure de l'observation**, sa
**position**, puis la météo : direction et vitesse du vent **dans l'unité que le
rapport lui-même indique**, température de l'air et du point de rosée, pression,
nuage, visibilité, temps présent, et — pour un navire — cap, vitesse, mer et
houle.

**La ligne d'origine est conservée au-dessus du décodage.** C'est délibéré. Si vous
voulez vérifier un nombre, vous voyez les chiffres dont il vient ; et si vous ne
y croyez pas, vous avez raison de le dire.

**La barre oblique est une réponse.** Les groupes comme `8597/` signifient *non
transmis* — ce qui n'est ni zéro ni une absence. Ils s'affichent **non transmis**
plutôt que vides, car une case vide laisserait croire à notre faute.

Tout ce que le décodeur ne reconnaît pas est conservé et affiché dans les mots
du station. Ceux qui saisissent leurs rapports à la main y mettent la partie la
plus intéressante, et c'est celle qui vaut la peine d'être lue.

Les rapports ne sont conservés que pour la session. La liste repart vide demain —
un rapport lu demain serait un mensonge sur son heure d'observation.

---

## Ce que ce n'est pas

**Ce n'est pas une prévision.** C'est une observation, d'un seul navire, à un seul
instant. Pour préparer une traversée, c'est un élément parmi d'autres, et non celui
sur lequel décider.

**Cela ne remplace pas le service officiel.** Ceux qui.ensure ces émissions le font
par sécurité. Lire le rapport, c'est le plaisir.

---

## S'il ne se passe rien

| Ce que vous voyez | Ce que vous pouvez essayer |
|---|---|
| Aucun son | Vérifiez la fréquence et le mode |
| Son mais aucun caractère | Le shift est probablement 170 et non 425/450 — ici c'est **170** |
| Caractères mais incompréhensibles | Généralement la **bande latérale** — essayez USB |
| Caractères, pas de bouton WEATHER | La ligne n'était pas un rapport codé. La météo et le RTTY ordinaire partagent la bande. |
| Bouton WEATHER, chiffres faux | **Dites-le nous.** C'est le bug que nous cherchons. |

La dernière ligne est la plus utile. Un nombre faux vaut mieux que vingt bons, et
nous ne pouvons pas le trouver sans quelqu'un qui écoute.

---

## Pour aller plus loin

- [`USER_MANUAL.md` §3.3a](USER_MANUAL.md) — la référence complète de ce décodeur.
- `docs/synop-notes.md` dans le dépôt — d'où vient le format, et ce qui reste incertain.