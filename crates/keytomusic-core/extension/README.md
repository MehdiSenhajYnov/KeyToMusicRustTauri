# Extension KeyToMusic Mood AI

Envoie les pages du manga que tu lis à l'application KeyToMusic (API locale
`http://127.0.0.1:8765`), qui découpe le chapitre en scènes avec un modèle de
vision et lance la musique associée à chaque ambiance.

## Installation

L'application exporte ce dossier (Mood AI → « Installer l'extension »).

- **Chrome, Edge, Brave, Opera** : `chrome://extensions` → activer le *mode
  développeur* → *Charger l'extension non empaquetée* → choisir le dossier.
- **Firefox** : `about:debugging#/runtime/this-firefox` → *Charger un module
  complémentaire temporaire* → choisir `manifest.json` (à refaire à chaque
  redémarrage de Firefox tant que l'extension n'est pas signée). Si Firefox
  demande l'accès aux sites, l'accepter.

## Fonctionnement

- Sur n'importe quel site, l'extension cherche une colonne d'au moins trois
  grandes images (le lecteur). La première fois sur un site, elle demande s'il
  faut l'analyser ; le choix se change dans la fenêtre de l'extension.
- Les pages sont capturées depuis la page (les images déjà chargées), sinon
  rechargées par l'extension, réduites à 900 px de large et envoyées à l'app.
- La page à l'écran est signalée à l'app : elle analyse d'abord autour d'elle et
  joue l'ambiance de sa scène. L'icône affiche l'ambiance en cours.

Les lecteurs qui affichent une seule page à la fois, ou qui dessinent les pages
dans un `<canvas>`, ne sont pas pris en charge.

## API (app)

| Méthode | Chemin | Corps |
|---|---|---|
| GET | `/api/status` | — |
| POST | `/api/chapter` | `{chapter, title?, totalPages?}` → `{received: [pages]}` |
| POST | `/api/pages` | `{chapter, pages: [{page, data}]}` (base64) |
| POST | `/api/position` | `{chapter, page, totalPages?}` → `{current, analyzed, …}` |

Les pages sont numérotées à partir de 0.
