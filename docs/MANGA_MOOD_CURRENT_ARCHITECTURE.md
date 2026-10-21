# Manga Mood Current Architecture

> Statut: reference produit actuelle au 2026-03-28.
>
> Si une autre doc contredit ce fichier sur le flux extension/backend/runtime, ce fichier fait foi.

## Scope

Ce document decrit le systeme Manga Mood actuellement branche dans l'application et dans l'extension navigateur.

Il ne couvre pas :

- les anciens pipelines 10 moods
- les anciens endpoints HTTP retires
- les benchmarks historiques comme source de verite produit

## Runtime actif

- Modele runtime: `Qwen3-VL-4B-Thinking`
- Noyau partage de prediction page: [winner.rs](/home/mehdi/Dev/KeyToMusicRustTauri/src-tauri/src/mood/winner.rs)
- Scheduler produit chapitre: [chapter_pipeline.rs](/home/mehdi/Dev/KeyToMusicRustTauri/src-tauri/src/mood/chapter_pipeline.rs)
- API HTTP locale: [server.rs](/home/mehdi/Dev/KeyToMusicRustTauri/src-tauri/src/mood/server.rs)
- Cache en memoire par chapitre: [cache.rs](/home/mehdi/Dev/KeyToMusicRustTauri/src-tauri/src/mood/cache.rs)
- Couche de commit OST: [soundtrack_commit.rs](/home/mehdi/Dev/KeyToMusicRustTauri/src-tauri/src/mood/soundtrack_commit.rs)

Le systeme actif est base sur 8 moods:

- `epic`
- `tension`
- `sadness`
- `comedy`
- `romance`
- `horror`
- `peaceful`
- `mystery`

Chaque prediction transporte aussi une intensite `1..3`.

## Idee cle

Le produit ne travaille plus avec une seule notion de "mood".

Il y a maintenant 2 couches distinctes :

1. `page mood`
2. `OST committed mood`

### `page mood`

Le `page mood` est la prediction finale page par page sortie par le pipeline courant.

Il est calcule a partir de :

- l'analyse visible-window pour la page courante
- ou le pipeline chapitre
- plus les post-traitements deja presents dans `winner.rs` et `chapter_pipeline.rs`

Ce resultat est celui qui est stocke dans le cache, benchmarke dans les fichiers Production, et visible comme base dans RealTest.

### `OST committed mood`

Le `OST committed mood` est l'etat musical final decide pour la lecture audio.

Il est derive du `page mood`, mais il n'est pas autorise a changer d'OST pour un pic isole d'une seule page.

Cette couche existe parce qu'un faux positif page par page est acceptable pour l'analyse, mais pas pour la musique :

- un lecteur manga peut supporter une page mal classee
- une OST qui change 10 secondes puis revient casse l'experience

## Pourquoi cette couche existe

Le pipeline page par page peut encore produire des pics isoles incorrects.

Exemple concret:

- run Production courant: `BL/2/21 = tension:2`, `BL/2/22 = comedy:3`, `BL/2/23 = tension:2`
- dans le contexte de lecture et de musique, ce `comedy` central est un spike aberrant
- si on relance une OST `comedy` pour cette page puis qu'on revient a `tension`, on cree une rupture artificielle

Le but de la couche `soundtrack_commit` est donc :

- conserver la finesse analytique du pipeline page par page
- mais rendre les transitions OST beaucoup plus stables
- sans revenir a l'ancien `MoodDirector`

## Couche OST commit

### Position dans le flux

La couche OST ne remplace pas le pipeline page par page.

Elle est appliquee apres resolution d'une page, au moment ou l'extension demande au backend de rejouer le mood final via `POST /api/trigger`.

Le flux live est donc :

1. le backend resout une page et produit un `page mood`
2. l'extension appelle `POST /api/trigger` avec `mood + chapter + page`
3. le backend relit l'entree de cache courante
4. le backend regarde aussi jusqu'a `page+2` si ces pages sont deja en cache
5. `soundtrack_commit` decide si l'OST doit rester ou changer
6. le backend emet `mood_detected` puis `mood_committed`

### Familles musicales

La couche OST raisonne d'abord en familles, pas seulement en labels bruts :

- `Action` = `epic + tension`
- `Emotion` = `sadness + romance`
- `Light` = `comedy + peaceful`
- `Suspense` = `horror + mystery`

Consequence produit :

- un switch entre familles est couteux et demande plus de confirmation
- un changement dans la meme famille peut etre accepte plus facilement

Exemple :

- `tension -> epic` peut etre une intensification de la meme OST d'action
- `tension -> comedy` est un vrai changement de couleur musicale, donc il faut plus de certitude

### Fenetre de contexte

La logique actuelle utilise une petite fenetre locale autour de la page courante :

- historique recent conserve: jusqu'a `6` observations
- rayon de contexte pour la decision: `-2 .. +2`
- futur live exploite si deja connu en cache: jusqu'a `page+2`

Le score de contexte est pondere :

- la page courante pese le plus
- `+/-1` pese moins
- `+/-2` pese encore moins

Le but est de decider "comme le modele", c'est-a-dire avec du contexte avant/apres, au lieu de faire une simple hysteresis causale qui retarde tout d'une page.

### Regles de commit

La couche OST suit 4 principes :

1. Un spike cross-family d'une seule page ne doit pas changer l'OST.
2. Un vrai nouveau segment confirme par le voisinage peut changer l'OST des la premiere page du segment.
3. Un changement dans la meme famille est plus permissif qu'un changement cross-family.
4. Si le futur n'est pas encore en cache, la page courante peut quand meme debloquer un switch immediat si ses scores sont tres forts et que le role narratif n'est pas un simple `continuation`.

En plus de ces regles, la logique actuelle applique maintenant un decrochage explicite pour la famille `Light` :

- si l'OST engagee est `Light`
- si la page courante a deja quitte `Light`
- et que `page+1` et `page+2` ont aussi quitte `Light`
- alors on n'a plus le droit de rester colle a cette OST `Light`

Dans ce cas, la couche OST force la sortie de la famille courante, puis choisit la nouvelle cible a partir du contexte local au lieu de reprendre aveuglement le label brut de la page.

En pratique, le systeme essaye d'eviter 2 ecueils :

- faux positif `comedy` d'une seule page au milieu d'une zone action
- delai mecanique d'une page sur tous les vrais changements
- collage artificiel d'une OST `Light` alors que les pages suivantes disent deja clairement "on n'est plus la"

### Exemple `BL/2/22`

Cas Production observe :

- `BL/2/21 = tension:2`
- `BL/2/22 = comedy:3`
- `BL/2/23 = tension:2`

Interpretation OST :

- le `page mood` reste `comedy:3`
- mais l'`OST committed mood` reste `tension:2`

Pourquoi :

- `comedy` appartient a la famille `Light`
- les pages voisines soutiennent `Action`
- le switch `Action -> Light` n'a pas assez de support de contexte
- la couche OST refuse donc de relancer une OST `comedy` pour une seule page

Ce comportement est exactement la correction voulue pour eviter les micro-switchs musicaux.

### Exemple `BL/3/09..12`

Cas Production observe :

- `BL/3/08 = comedy:3`
- `BL/3/09 = tension:2`
- `BL/3/10 = horror:3`
- `BL/3/11 = tension:2`
- `BL/3/12 = horror:3`

Probleme produit :

- l'ancienne version de `soundtrack_commit` pouvait rester bloquee sur `comedy`
- comme `Action` et `Suspense` se partageaient le contexte, aucune famille adverse ne "gagnait proprement"
- resultat : l'OST `Light` survivait alors que la scene etait deja sortie de cette ambiance

Correction actuelle :

- si la page courante et `page+1/page+2` ont toutes quitte `Light`, on force la sortie de `Light`
- la nouvelle famille cible est choisie par le score de contexte local
- le nouveau mood cible est choisi dans cette famille avec une priorite musicale stable

Exemple :

- dans une famille `Action`, on prefere `tension` a `epic` en cas d'egalite
- dans une famille `Suspense`, on prefere `mystery` a `horror` en cas d'egalite

Le but n'est pas de corriger le `page mood` semantique a tout prix.  
Le but est d'eviter qu'une OST absurde reste engagee juste parce que le bruit se repartit entre deux alternatives serieuses.

## Ancien `MoodDirector`

L'ancien `MoodDirector` existe encore dans le repo pour reference et anciens tests, dans [director.rs](/home/mehdi/Dev/KeyToMusicRustTauri/src-tauri/src/mood/director.rs).

Il n'est pas branche dans le flux produit actuel.

Le runtime actif utilise :

- `winner.rs`
- `chapter_pipeline.rs`
- `soundtrack_commit.rs`

Il ne faut donc pas confondre :

- ancien `MoodDirector` legacy
- nouvelle couche `soundtrack_commit` branchee en live

## Composants

### App Tauri

L'app Tauri fait tourner :

- le runtime `llama-server`
- l'API HTTP locale
- le cache mood en memoire
- le pipeline chapitre
- la couche de commit OST appelee au moment de `POST /api/trigger`

### Extension navigateur

L'extension n'essaie plus de "decider" le mood elle-meme.

Elle fait 4 choses :

1. detecter la page visible et l'ordre de lecture
2. precharger les pages voisines dans le navigateur reel de l'utilisateur
3. pousser les pages vers l'API locale, puis consommer le cache
4. demander explicitement le commit OST via `POST /api/trigger`

### Session navigateur reelle

Le transport image est base sur le navigateur reel de l'utilisateur.

Pourquoi :

- les `fetch` directs sur les URLs images sont souvent bloques par Cloudflare ou par des contraintes de session
- le site lecteur, lui, sait souvent charger les images correctement dans son propre contexte

La strategie active est donc :

- prechargement dans la page
- capture des images chargees en base64
- envoi base64 au backend

## Endpoints actifs

L'API HTTP locale expose actuellement :

- `POST /api/analyze-window`
- `POST /api/chapter/page`
- `POST /api/chapter/focus`
- `POST /api/live/cancel`
- `POST /api/trigger`
- `POST /api/lookup`
- `GET /api/cache/status`
- `GET /api/status`
- `GET /api/moods`

Notes importantes :

- l'ancien `POST /api/analyze` ne fait plus partie du flux extension
- l'ancien helper Tauri local `analyze_mood(image_path)` a ete retire du produit

## Workflow actuel

### 1. Focus utilisateur

Le content script detecte la page visible `X`, l'ordre de lecture et le chapitre courant.

Il envoie un hint de focus au backend via `chapter_focus`.

### 2. Buffer cible

Le systeme vise un buffer d'analyse :

- `X-10 .. X+20`

Ce buffer est la zone qu'on veut idealement avoir deja analysee et disponible en cache.

### 3. Fenetre reellement chargee

Le pipeline backend a besoin de contexte autour des pages cibles.

L'extension charge donc une fenetre plus large :

- `X-14 .. X+24`

Raison :

- le pipeline a besoin d'une marge technique d'environ `+/-4` pages pour pouvoir publier une page cible

En pratique :

- zone analysee voulue: `X-10 .. X+20`
- zone brute a charger/envoyer: `X-14 .. X+24`

### 4. Prechargement in-page

Le content script lit les vraies sources des images depuis le DOM du reader :

- `src`
- `srcset`
- `data-src`
- `data-srcset`
- autres variantes lazy connues

Il force ensuite le chargement :

- si possible directement sur la balise `img` du reader
- sinon via un buffer cache hors ecran dans la page

Une fois une image reellement chargee :

- elle devient capturable en base64
- elle peut etre poussee au backend

### 5. Alimentation du chapitre

Les pages chargees dans la fenetre `X-14 .. X+24` sont envoyees a :

- `POST /api/chapter/page`

Le backend enregistre ces pages dans le pipeline chapitre et priorise le calcul autour du focus courant plutot que de bloquer sur la page `0`.

### 6. Resolution de la page visible

Pour la page visible `X`, le flux prioritaire est :

1. `POST /api/lookup`
2. si miss cache, `POST /api/analyze-window`
3. puis `POST /api/trigger` pour decider l'etat OST reel

`analyze-window` envoie une fenetre locale centree sur la page visible, jusqu'a `9` pages max (`-4 .. +4`) si elles sont capturables.

Le strict minimum pour lancer ce chemin est la disponibilite locale de `X-2 .. X+2`.

### 7. Evenements emis

Apres `POST /api/trigger`, le backend emet 2 evenements distincts :

- `mood_detected`
- `mood_committed`

Semantique :

- `mood_detected` = resultat brut de la page courante
- `mood_committed` = etat musical final apres smoothing OST

Consequence debug importante :

- un `mood_detected = comedy`
- suivi d'un `mood_committed = tension`

est un comportement normal si la couche OST a filtre un spike isole.

### 8. Annulation

Si l'utilisateur change de page pendant une analyse visible :

- l'extension annule la requete HTTP en cours
- le background notifie aussi le backend via `POST /api/live/cancel`

Le but est de ne pas bloquer la priorite utilisateur sur une ancienne page visible.

## Priorites de scheduling

La priorite produit est centree utilisateur :

- page visible `X`
- puis le buffer autour de `X`, avec biais vers l'avant

La logique de priorite extension/popup est de type :

- `X`
- `X+1`
- `X+2`
- `X-1`
- `X+3`
- `X+4`
- `X-2`
- etc.

Le backend chapitre ne suit plus un ordre strict `0 -> fin`.

## Cache et statuts

### Cache

Le cache mood est :

- en memoire
- par chapitre
- vide a chaque changement de chapitre

Une entree de cache contient :

- mood
- intensite
- scores
- role narratif
- source (`visible_window` ou `chapter_pipeline`)
- statut `finalized`

Important :

- le cache stocke le `page mood`
- il ne stocke pas directement l'etat musical commute final
- la couche OST lit le cache puis decide le `committed mood` a la demande sur `POST /api/trigger`

### `/api/cache/status`

Cet endpoint est la source de debug pour l'extension. Il expose notamment :

- pages prêtes en cache
- pages deja enregistrees dans le pipeline
- focus courant
- phase backend active
- derniere erreur backend

## Extension UI et debug

Le popup actuel est un outil de debug produit. Il montre :

- statut du service
- page visible
- dernier mood
- nombre de pages deja analysees dans la cible
- travail courant
- pages deja prêtes
- prochaines pages attendues
- evenements recents

Il expose aussi des toggles distincts :

- detection globale
- badge/page overlay
- playback trigger

Le toggle `playback trigger` coupe la republication musicale sans couper l'analyse du pipeline.

## RealTest et comparaison base vs smoothing

Le benchmark Production continue de produire le resultat page par page de reference :

- [realtest_suite_t4b_wide5_selective_hold.json](/home/mehdi/Dev/KeyToMusicRustTauri/manga-mood-ai/results/realtests/realtest_suite_t4b_wide5_selective_hold.json)

Le benchmark genere aussi un companion derive avec la couche OST appliquee :

- [realtest_suite_t4b_wide5_selective_hold_soundtrack_commit.json](/home/mehdi/Dev/KeyToMusicRustTauri/manga-mood-ai/results/realtests/realtest_suite_t4b_wide5_selective_hold_soundtrack_commit.json)

Dans le viewer RealTest :

- le resultat base correspond au `page mood`
- la variante `+ OST commit` correspond au `committed mood`
- la `Reference` ground truth peut aussi etre comparee a `Reference + OST commit`
- le bouton `Compare OST` compare les deux directement

Ce viewer sert a verifier qu'on filtre bien les spikes du type `BL/2/22 = comedy` sans rendre tout le systeme trop collant ailleurs.

Il sert aussi a verifier que la couche OST :

- ne garde pas `comedy` quand `page+1/page+2` montrent deja une vraie sortie de `Light`
- choisit une cible musicale plus stable que le simple label brut de la page

## Limites connues

- Le systeme ne peut precharger que les pages dont la source est deja recuperable depuis la page lecteur ou son JS.
- Si le site ne revele pas encore les URLs futures, l'extension ne peut pas inventer les pages lointaines.
- Le chemin `analyze-window` reste conditionne par la capturabilite locale de `X-2 .. X+2`.
- Si `page+1` et `page+2` ne sont pas encore en cache, la couche OST decide avec moins de futur disponible.
- Le cache est memoire seulement; il sert le chapitre courant, pas une persistence cross-session.

## Fichiers de reference

- [src-tauri/src/mood/server.rs](/home/mehdi/Dev/KeyToMusicRustTauri/src-tauri/src/mood/server.rs)
- [src-tauri/src/mood/soundtrack_commit.rs](/home/mehdi/Dev/KeyToMusicRustTauri/src-tauri/src/mood/soundtrack_commit.rs)
- [src-tauri/src/mood/chapter_pipeline.rs](/home/mehdi/Dev/KeyToMusicRustTauri/src-tauri/src/mood/chapter_pipeline.rs)
- [src-tauri/src/mood/winner.rs](/home/mehdi/Dev/KeyToMusicRustTauri/src-tauri/src/mood/winner.rs)
- [src-tauri/src/mood/director.rs](/home/mehdi/Dev/KeyToMusicRustTauri/src-tauri/src/mood/director.rs)
- [WebExtension/manga-mood/content.js](/home/mehdi/Dev/KeyToMusicRustTauri/WebExtension/manga-mood/content.js)
- [WebExtension/manga-mood/background.js](/home/mehdi/Dev/KeyToMusicRustTauri/WebExtension/manga-mood/background.js)
- [WebExtension/manga-mood/popup.js](/home/mehdi/Dev/KeyToMusicRustTauri/WebExtension/manga-mood/popup.js)
- [manga-mood-ai/results/realtest-results-viewer.js](/home/mehdi/Dev/KeyToMusicRustTauri/manga-mood-ai/results/realtest-results-viewer.js)
