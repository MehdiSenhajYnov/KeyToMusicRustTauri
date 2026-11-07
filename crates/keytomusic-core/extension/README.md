# Extension KeyToMusic Mood AI

Fait jouer à l'application KeyToMusic (API locale `http://127.0.0.1:8765`) la
musique qui colle à la scène du manga que tu lis : l'app découpe le chapitre en
scènes avec un modèle de vision et lance les sons de l'ambiance de chaque scène.

## Installation

L'application exporte ce dossier (Mood AI → « Installer l'extension »).

- **Chrome, Chromium, Edge, Brave, Opera** : `chrome://extensions` → activer
  le *mode développeur* → *Charger l'extension non empaquetée* → choisir le
  dossier.
- **Firefox** : `about:debugging#/runtime/this-firefox` → *Charger un module
  complémentaire temporaire* → choisir `manifest.json` (à refaire à chaque
  redémarrage de Firefox tant que l'extension n'est pas signée).

Les mises à jour sont automatiques : au démarrage, l'app réécrit ce dossier si
elle embarque une version plus récente, et l'extension se recharge d'elle-même
quand l'app la lui annonce.

## Fonctionnement

- L'extension reconnaît le lecteur du site :
  - **ts_reader** (sushiscan et la plupart des sites WordPress de scans) : la
    liste des pages est lue dans le script de la page, en mode « une page à la
    fois » comme en lecture verticale ;
  - **Madara** (`chapter_preloaded_images`) ;
  - sinon, toute colonne d'au moins trois grandes images.
- La première fois sur un site, elle demande s'il faut l'utiliser ; le choix se
  change dans la fenêtre de l'extension.
- Elle envoie à l'app l'adresse de chaque page : **l'app les télécharge
  elle-même** (avec la page du lecteur comme Referer), sans dépendre de ce que
  le site laisse lire à une extension. Les pages que l'app n'arrive pas à
  télécharger (`blob:`, hôte qui la refuse), l'extension les capture et les
  envoie.
- Elle signale la page à l'écran : l'app analyse d'abord autour d'elle et joue
  l'ambiance de sa scène. L'icône affiche l'ambiance en cours.

## API (app)

| Méthode | Chemin | Corps → réponse |
|---|---|---|
| GET | `/api/status` | → `{enabled, configured, model, extensionVersion, chapter, …}` |
| POST | `/api/chapter` | `{chapter, title?, totalPages?, url?, pages?: [URL]}` → `{received, wanted}` |
| POST | `/api/pages` | `{chapter, pages: [{page, data}]}` (base64) → `{received, rejected}` |
| POST | `/api/position` | `{chapter, page, totalPages?}` → `{current, analyzed, wanted, …}` |

Les pages sont numérotées à partir de 0. `wanted` liste les pages que l'app
attend de l'extension.
