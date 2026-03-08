# Manga Mood Current Architecture

> Statut: reference produit actuelle au 2026-03-08.
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
- Noyau partage: [winner.rs](/home/mehdi/Dev/KeyToMusicRustTauri/src-tauri/src/mood/winner.rs)
- Scheduler produit: [chapter_pipeline.rs](/home/mehdi/Dev/KeyToMusicRustTauri/src-tauri/src/mood/chapter_pipeline.rs)
- API HTTP locale: [server.rs](/home/mehdi/Dev/KeyToMusicRustTauri/src-tauri/src/mood/server.rs)
- Cache en memoire par chapitre: [cache.rs](/home/mehdi/Dev/KeyToMusicRustTauri/src-tauri/src/mood/cache.rs)

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

## Composants

### App Tauri

L'app Tauri fait tourner :

- le runtime `llama-server`
- l'API HTTP locale
- le cache mood en memoire
- le `MoodDirector` qui declenche les changements d'OST cote app

### Extension navigateur

L'extension n'essaie plus de "decider" le mood elle-meme. Elle fait 3 choses :

1. detecter la page visible et l'ordre de lecture
2. precharger les pages voisines dans le navigateur reel de l'utilisateur
3. pousser les pages vers l'API locale, puis consommer le cache

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
- `analyze_mood(image_path)` existe encore comme commande Tauri locale, mais c'est un helper interne/manuel, pas le chemin de production de l'extension

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

`analyze-window` envoie une fenetre locale centree sur la page visible, jusqu'a `9` pages max (`-4 .. +4`) si elles sont capturables.

Le strict minimum pour lancer ce chemin est la disponibilite locale de `X-2 .. X+2`.

### 7. Annulation

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

### `/api/cache/status`

Cet endpoint est la source de debug pour l'extension. Il expose notamment :

- pages prêtes en cache
- pages deja enregistrees dans le pipeline
- focus courant
- phase backend active
- derniere erreur backend

## Semantique du popup extension

Le popup simplifie volontairement l'etat en 3 notions :

- `Already Analyzed`
- `Current Work`
- `Next Pages`

### Already Analyzed

Pages du buffer cible `X-10 .. X+20` qui ont deja un mood pret en cache.

### Current Work

Ce champ est oriente page cible utilisateur, pas operation technique brute.

Exemple :

- si le backend calcule une fenetre centree sur `24` pour debloquer `22`
- le popup affiche `Analyzing Page 22`
- la note precise que le contexte centré sur `24` est en cours de calcul

### Next Pages

Pages encore manquantes dans la zone cible `X-10 .. X+20`.

### Etats frequents

- `Idle` : rien a faire cote extension et aucun travail cible immediat
- `Waiting for context` : des pages manquantes existent encore, mais le backend n'a pas encore assez de contexte local pour publier la prochaine page cible
- `Preparing Page X` : l'extension pousse encore des sources vers le pipeline
- `Analyzing Page X` : le backend travaille activement pour debloquer la prochaine page cible

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

Les anciens champs debug plus techniques (`Warming Pages`, `Prefetch Queue`, `Visible Queue`) ne sont plus la vue principale a lire.

## Limites connues

- Le systeme ne peut precharger que les pages dont la source est deja recuperable depuis la page lecteur ou son JS.
- Si le site ne revele pas encore les URLs futures, l'extension ne peut pas inventer les pages lointaines.
- Le chemin `analyze-window` reste conditionne par la capturabilite locale de `X-2 .. X+2`.
- Le cache est memoire seulement; il sert le chapitre courant, pas une persistence cross-session.

## Fichiers de reference

- [src-tauri/src/mood/server.rs](/home/mehdi/Dev/KeyToMusicRustTauri/src-tauri/src/mood/server.rs)
- [src-tauri/src/mood/chapter_pipeline.rs](/home/mehdi/Dev/KeyToMusicRustTauri/src-tauri/src/mood/chapter_pipeline.rs)
- [src-tauri/src/mood/winner.rs](/home/mehdi/Dev/KeyToMusicRustTauri/src-tauri/src/mood/winner.rs)
- [WebExtension/manga-mood/content.js](/home/mehdi/Dev/KeyToMusicRustTauri/WebExtension/manga-mood/content.js)
- [WebExtension/manga-mood/background.js](/home/mehdi/Dev/KeyToMusicRustTauri/WebExtension/manga-mood/background.js)
- [WebExtension/manga-mood/popup.js](/home/mehdi/Dev/KeyToMusicRustTauri/WebExtension/manga-mood/popup.js)
