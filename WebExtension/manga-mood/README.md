# KeyToMusic Manga Mood Extension

## Scope

Cette extension est le client navigateur du systeme Manga Mood de KeyToMusic.

Elle :

- detecte la page manga visible
- precharge des pages autour du lecteur dans la page elle-meme
- capture les images en base64 depuis le navigateur reel de l'utilisateur
- alimente le cache chapitre du backend local
- demande un fallback direct pour la page visible si le cache est encore froid

La doc canonique du flux complet est [docs/MANGA_MOOD_CURRENT_ARCHITECTURE.md](/home/mehdi/Dev/KeyToMusicRustTauri/docs/MANGA_MOOD_CURRENT_ARCHITECTURE.md).

## Pourquoi l'extension capture en base64

L'extension n'utilise pas le chemin "URL brute -> fetch backend" comme flux principal.

Raison :

- certains readers manga sont proteges par Cloudflare ou par des contraintes de session
- l'URL image peut etre bloquee hors du contexte de lecture reel

Le systeme actuel privilegie donc :

1. charger l'image dans la page lecteur
2. la capturer en base64
3. l'envoyer au backend local

## Workflow actuel

### Page visible

Pour la page visible `X` :

1. le content script envoie un `lookup`
2. si la page est deja en cache, l'app rejoue le mood immediatement
3. sinon l'extension lance `analyze-window` sur la page visible avec son contexte local

### Prise d'avance

L'extension vise un buffer analyse :

- `X-10 .. X+20`

Pour y arriver, elle force le chargement d'une fenetre plus large :

- `X-14 .. X+24`

Cette marge supplementaire sert uniquement a donner assez de contexte au pipeline backend.

### Prechargement in-page

Le content script exploite les vraies sources du reader :

- `src`
- `srcset`
- `data-src`
- `data-srcset`
- attributs lazy similaires

Il essaie d'abord de faire charger les images sur les vraies balises `img` du lecteur, puis tombe sur un buffer cache dans la page si necessaire.

## API locale utilisee

L'extension parle a l'API locale exposee par l'app Tauri :

- `POST /api/lookup`
- `POST /api/analyze-window`
- `POST /api/chapter/page`
- `POST /api/chapter/focus`
- `POST /api/live/cancel`
- `GET /api/cache/status`
- `GET /api/status`

## Debug popup

Le popup montre une vue simplifiee du systeme :

- `Already Analyzed` : pages deja prêtes dans la cible `X-10 .. X+20`
- `Current Work` : prochaine page cible que le systeme essaie de debloquer
- `Next Pages` : pages encore manquantes dans cette cible
- `Current Note` : detail technique utile, par exemple une fenetre de contexte centree sur une autre page

Exemple :

- `Current Work = Analyzing Page 22`
- `Current Note = Computing the context window centered on Page 24 to finish Page 22`

Cela signifie :

- le but utilisateur est bien la page `22`
- l'operation technique backend en cours utilise encore du contexte autour de `24`

## Installation

### Chrome / Chromium

1. Ouvrir `chrome://extensions`
2. Activer le mode developpeur
3. Cliquer sur `Load unpacked`
4. Selectionner `WebExtension/manga-mood`

### Firefox

1. Ouvrir `about:debugging#/runtime/this-firefox`
2. Cliquer sur `Load Temporary Add-on`
3. Selectionner `WebExtension/manga-mood/manifest.json`

## Sites actuellement declares

Les host permissions actuelles couvrent notamment :

- `sushiscan.net`
- `sushiscan.com`
- `lelmanga.com`
- `mangascan.cc`
- `scantrad.net`
- `mangakakalot.com`
- `manganato.com`
- `chapmanganato.to`

La liste exacte est dans [manifest.json](/home/mehdi/Dev/KeyToMusicRustTauri/WebExtension/manga-mood/manifest.json).

## Limitations

- L'extension ne peut precharger que les pages dont la source est deja discoverable depuis le DOM ou le JS du reader.
- Si le site ne revele pas les images futures, le prechargement lointain restera limite.
- Le flux visible direct a besoin que `X-2 .. X+2` soient capturables localement pour lancer `analyze-window`.

## Fichiers cles

- [content.js](/home/mehdi/Dev/KeyToMusicRustTauri/WebExtension/manga-mood/content.js)
- [background.js](/home/mehdi/Dev/KeyToMusicRustTauri/WebExtension/manga-mood/background.js)
- [popup.js](/home/mehdi/Dev/KeyToMusicRustTauri/WebExtension/manga-mood/popup.js)
- [manifest.json](/home/mehdi/Dev/KeyToMusicRustTauri/WebExtension/manga-mood/manifest.json)
