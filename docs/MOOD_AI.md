# Mood AI

Mood AI lance la musique qui colle à la scène du manga que tu lis. Un modèle
de vision (GPT par défaut, ou tout service compatible OpenAI) lit les pages du
chapitre ouvert dans le navigateur, découpe le chapitre en **scènes** et donne à
chacune une ambiance. Quand la page à l'écran change de scène, l'app joue les
touches étiquetées avec cette ambiance.

Le moteur vit dans `crates/keytomusic-core` et sert les deux applications
(Tauri et Awra).

## Parcours

```
Navigateur (extension)                 KeyToMusic (127.0.0.1:8765)                 Modèle (API OpenAI)
──────────────────────                 ───────────────────────────                 ───────────────────
chapitre détecté ── POST /api/chapter ─▶ chapitre courant (pages déjà reçues ◀─┐
pages capturées ─── POST /api/pages ───▶ pages (JPEG ≤ 900 px)                 │
page à l'écran ──── POST /api/position ▶ focus ── lot de 10 pages ─────────────┼─▶ scènes JSON
                                         libellés par page (cache disque) ◀────┘
                                         la scène change ─▶ mood_committed ─▶ touches jouées
```

1. **Extension** (`crates/keytomusic-core/extension/`) : sur n'importe quel site,
   elle cherche une colonne d'au moins trois grandes images (le lecteur). La
   première fois sur un site elle demande l'autorisation. Elle capture les pages
   déjà chargées (canvas), sinon les recharge elle-même, et signale la page à
   l'écran pendant le défilement. L'icône affiche l'ambiance en cours.
2. **API locale** (`mood/server.rs`) : 4 routes (`status`, `chapter`, `pages`,
   `position`), démarrée avec l'app quand Mood AI est activé.
3. **Ordonnancement** (`mood/chapter.rs`) : la page lue d'abord, puis la suite,
   puis ce qui précède. Lots de 10 pages consécutives, avec les 2 pages
   précédentes en contexte (et leur ambiance si elle est déjà connue). Deux
   requêtes en parallèle. Si des pages arrivent encore, attente de 1,5 s pour
   remplir le lot. Après une erreur : pause exponentielle (2 s → 60 s) ; une clé
   refusée ou un modèle inconnu bloque l'analyse jusqu'à de nouveaux réglages.
4. **Modèle** (`mood/analyzer.rs`, `mood/provider.rs`) : un prompt système
   décrit les 8 ambiances et les règles (une scène dure en général 3 pages ou
   plus ; un gag isolé dans un combat reste dans le combat ; lire les bulles ; les
   couvertures prennent l'ambiance voisine). La réponse est imposée par un schéma
   JSON strict : `{scenes: [{first_page, last_page, mood, intensity, summary}]}`.
   Si le service refuse `reasoning_effort` ou le schéma JSON, la requête est
   refaite sans (mode JSON simple, puis texte).
5. **Lecture** (`mood/engine.rs`, `session/mod.rs`) : quand la page à l'écran
   change de scène, `mood_detected` puis `mood_committed` sont émis. Pour chaque
   piste, la touche de cette ambiance dont l'intensité minimale est la plus haute
   sans dépasser l'intensité de la scène est jouée. Un changement d'intensité qui
   garde la même touche ne relance pas la musique.

Les scènes décidées sont gardées dans `data/cache/mood_scenes.json` (200
chapitres) : relire un chapitre ne coûte rien.

## Réglages

| Réglage | Où | Défaut |
|---|---|---|
| Activer | `moodAiEnabled` | non |
| Service | `moodBaseUrl` | `https://api.openai.com/v1` (préréglages : OpenAI, OpenRouter, Ollama, LM Studio) |
| Modèle | `moodModel` | `gpt-5-mini` (« Tester » liste les modèles du service) |
| Pages | `moodImageDetail` | `high` (le modèle lit les bulles) ; `low` = miniatures, moins cher |
| Port | `moodApiPort` | `8765` (le même que dans l'extension) |
| Clé API | fichier `data/mood_api_key` (droits 600) | sinon `KEYTOMUSIC_MOOD_API_KEY`, ou `OPENAI_API_KEY` pour OpenAI |

Les services locaux (`localhost`) n'ont pas besoin de clé.

## Coût et vitesse

Avec des pages détaillées, une page de manga vaut environ un millier de jetons
d'entrée. Un chapitre de 20 pages coûte donc environ 25 000 jetons (contexte
compris) ; avec un modèle « mini », quelques centimes au plus. Un lot de 10
pages répond en quelques secondes : la page lue est prête rapidement, le reste
du chapitre est analysé en avance.

## Limites

- Lecteurs qui affichent une seule page à la fois, ou dessinent les pages dans
  un `<canvas>` : non pris en charge.
- Firefox : l'extension non signée se recharge à chaque redémarrage
  (`about:debugging`).
- Les pages sont envoyées au service choisi (OpenAI par défaut).

## Évaluer

```bash
OPENAI_API_KEY=… cargo run --release -p keytomusic-core --example mood_eval -- \
    chemin/vers/RealTest --chapters BL,OP/1 --model gpt-5-mini --out resultats.json
```

Le jeu annoté RealTest (ancien dossier `manga-mood-ai/test-images/RealTest`)
reste dans l'historique git. Référence de l'ancien pipeline local (Qwen3-VL 4B +
règles) sur BL + OP/1-5 (380 pages) : 55 % de pages exactes, 76 % de bonne
famille, pour 44 % en répondant toujours « tension ».

## Tests

- `cargo test -p keytomusic-core mood` : prompt et lecture des réponses,
  ordonnancement, cache, extension exportée, conversion des images.
- `cargo test -p keytomusic-core --test mood_flow` : parcours complet (appels de
  l'extension → moteur → faux serveur OpenAI → événements de lecture).
