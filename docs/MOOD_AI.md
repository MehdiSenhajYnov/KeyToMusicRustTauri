# Mood AI

Mood AI lance la musique qui colle à la scène du manga que tu lis. Un modèle
de vision (GPT via ton abonnement ChatGPT par défaut, ou tout service compatible OpenAI avec une clé) lit les pages du
chapitre ouvert dans le navigateur, découpe le chapitre en **scènes** et donne à
chacune une ambiance. Quand la page à l'écran change de scène, l'app joue les
**sons** étiquetés avec cette ambiance.

Le moteur vit dans `crates/keytomusic-core` et sert les deux applications
(Tauri et Awra).

## Parcours

```
Navigateur (extension)                  KeyToMusic (127.0.0.1:8765)                    Modèle (API OpenAI)
──────────────────────                  ───────────────────────────                    ───────────────────
lecteur détecté ── POST /api/chapter ──▶ chapitre + URL des pages
  (liste des pages)                      téléchargement des pages (Referer = lecteur)
pages refusées à l'app ◀── wanted ────── pages introuvables
  ── POST /api/pages ──────────────────▶ pages (JPEG, tuiles pour les webtoons)
page à l'écran ─── POST /api/position ─▶ focus ── lot de 10 pages ─────────────────▶ scènes JSON
                                         libellés par page (cache disque) ◀──────────┘
                                         la scène change ─▶ mood_committed ─▶ sons joués
```

1. **Extension** (`crates/keytomusic-core/extension/`) : elle reconnaît le
   lecteur du site — **ts_reader** (sushiscan et la plupart des sites WordPress
   de scans, liste des pages lue dans le script de la page, en mode « page par
   page » comme en lecture verticale), **Madara** (`chapter_preloaded_images`),
   sinon toute colonne d'au moins trois grandes images. La première fois sur un
   site elle demande l'autorisation. Elle envoie **l'adresse** de chaque page et
   signale la page à l'écran (sélecteur de page, clic, défilement). L'icône
   affiche l'ambiance en cours.
2. **Téléchargement** (`mood/fetch.rs`, `mood/chapter.rs`, `mood/engine.rs`) :
   l'app télécharge elle-même les pages (4 à la fois, la page lue d'abord), avec
   la page du lecteur en Referer et son propre User-Agent (`KeyToMusic/…`). Les
   hébergeurs d'images refusent souvent les extensions (pas de CORS, canvas
   « contaminé ») et les requêtes sans Referer (anti-hotlink) : l'app n'a aucune
   de ces limites. Une page refusée deux fois (ou en `blob:`) est demandée à
   l'extension (`wanted`), qui la capture (canvas) ou la recharge avec le
   Referer du lecteur (règle `declarativeNetRequest` de session).
3. **Images** (`mood/image.rs`) : page ≤ 900 px de large ; double page jusqu'à
   1600 px ; bande de webtoon découpée en tuiles de 1400 px de haut (6 au plus,
   réparties sur la longueur) pour que le texte reste lisible.
4. **Ordonnancement** (`mood/chapter.rs`) : la page lue d'abord, puis la suite,
   puis ce qui précède. Lots de 10 pages consécutives, avec les 2 pages
   précédentes en contexte (et leur ambiance si elle est déjà connue). Deux
   requêtes en parallèle. Si des pages arrivent encore, attente de 1,5 s pour
   remplir le lot. Après une erreur : pause exponentielle (2 s → 60 s) ; une clé
   refusée ou un modèle inconnu bloque l'analyse jusqu'à de nouveaux réglages.
5. **Modèle** (`mood/analyzer.rs`, `mood/provider.rs`) : un prompt système
   décrit les 8 ambiances et les règles (une scène dure en général 3 pages ou
   plus ; un gag isolé dans un combat reste dans le combat ; lire les bulles ; les
   couvertures prennent l'ambiance voisine). La réponse est imposée par un schéma
   JSON strict : `{scenes: [{first_page, last_page, mood, intensity, summary}]}`.
   Si le service refuse `reasoning_effort` ou le schéma JSON, la requête est
   refaite sans (mode JSON simple, puis texte).
6. **Lecture** (`mood/engine.rs`, `session/mod.rs`, `session/moods.rs`) : quand
   la page à l'écran change de scène, `mood_detected` puis `mood_committed` sont
   émis. Voir « Ambiances des sons » ci-dessous.

Sur sushiscan (One Piece 1180, 15 pages) : pages téléchargées en ~1,5 s,
chapitre analysé en ~6 s, première musique ~5 s après l'ouverture.

## Ambiances des sons

L'ambiance (et l'intensité minimale) est portée par **chaque son**, quelle que
soit l'organisation du profil :

- touches groupées par ambiance (« Q = épique ») : associer la touche étiquette
  tous ses sons ;
- un son par touche : l'ambiance du son est celle de la touche ;
- touche qui mélange : elle répond à chaque ambiance avec les sons concernés.

Pour une scène, chaque piste prend les touches dont des sons de cette ambiance
atteignent le niveau le plus haut sans dépasser l'intensité de la scène, et ne
pioche que parmi ces sons (en suivant le mode de lecture de la touche ; une
boucle lancée par Mood AI reste dans l'ambiance). Si plusieurs touches sont à
égalité, une autre que la dernière est choisie. Ce qui joue déjà et convient
encore à la nouvelle scène continue sans être relancé.

Les champs `mood`/`moodIntensity` des touches sont une copie dérivée (ambiance
principale de leurs sons) pour l'interface web et les anciennes versions ; les
profils d'avant sont migrés au chargement (l'ambiance d'une touche passe à ses
sons). « Proposer des ambiances » (page Mood AI) fait étiqueter par le modèle les
sons de musique qui n'en ont pas, d'après leur titre, avec relecture avant
application.

## Extension : installation et mises à jour

L'app exporte l'extension (Mood AI → « Installer l'extension », par défaut dans
le dossier Téléchargements) ; on la charge une fois en « non empaquetée ». À
chaque démarrage, l'app réécrit les copies qu'elle a exportées si elles sont
plus anciennes, et l'extension se recharge d'elle-même quand l'app annonce une
version plus récente (`extensionVersion` dans `/api/status`).

Les scènes décidées sont gardées dans `data/cache/mood_scenes.json` (200
chapitres) : relire un chapitre ne coûte rien.

## Réglages

| Réglage | Où | Défaut |
|---|---|---|
| Activer | `moodAiEnabled` | non |
| Paiement | `moodAuth` | `chatgpt` (abonnement ChatGPT Plus/Pro, « Sign in with ChatGPT ») ; `api_key` |
| Service (clé API) | `moodBaseUrl` | `https://api.openai.com/v1` (préréglages : OpenAI, OpenRouter, Ollama, LM Studio) |
| Modèle | `moodModel` | `gpt-6.1-sol` (liste déroulante remplie avec les modèles du compte) |
| Pages | `moodImageDetail` | `high` (le modèle lit les bulles) ; `low` = miniatures, moins cher |
| Port | `moodApiPort` | `8765` (le même que dans l'extension) |
| Clé API | fichier `data/mood_api_key` (droits 600) | sinon `KEYTOMUSIC_MOOD_API_KEY`, ou `OPENAI_API_KEY` pour OpenAI |

Les services locaux (`localhost`) n'ont pas besoin de clé.

## Abonnement ChatGPT (« Sign in with ChatGPT »)

Flux officiel d'OpenAI pour les apps open source exécutées en local
(developers.openai.com/siwc/token-sharing-open-source), dans `mood/chatgpt.rs` :

1. « Continue with ChatGPT » ouvre `auth.openai.com/api/accounts/authorize` dans le
   navigateur (PKCE S256, `state`, `nonce`, scopes `openid profile email
   offline_access resource.invoke chatgpt.tokens.use.direct`, ressource
   `https://api.openai.com/v1`). La première fois, `client_id=dynamic_agent_client`
   + `agent_name_hint=KeyToMusic` inscrivent l'app pour ce compte ; l'identifiant
   renvoyé (`oaiapp_…`) est gardé et réutilisé. `ext_agent_host_id` (`urn:uuid:…`)
   identifie l'installation.
2. Le navigateur revient sur `http://127.0.0.1:1455/auth/callback` (autre port si
   pris) ; le code est échangé sans secret ; le jeton d'identité est vérifié
   (signature JWKS, émetteur, audience, expiration, nonce) et la permission
   `chatgpt.tokens.use.direct` contrôlée.
3. Les jetons sont dans `data/mood_chatgpt.json` (droits 600). Le jeton d'accès
   (1 h) est renouvelé automatiquement ; « Se déconnecter » révoque la session.
4. Les requêtes passent par `POST /v1/responses` (`store: false`, `stream: true`,
   prompt dans `instructions`, réponse en schéma JSON). Les erreurs du forfait
   (limite atteinte → pause de 15 min, compte non éligible, session révoquée)
   sont affichées telles quelles ; « Gérer l'utilisation » ouvre
   chatgpt.com/settings/usage.

Aucun mot de passe ne passe par KeyToMusic : la connexion se fait sur la page
d'OpenAI.

## Coût et vitesse

Avec des pages détaillées, une page de manga vaut environ un millier de jetons
d'entrée : un chapitre de 20 pages en consomme environ 25 000 (contexte compris).
Avec l'abonnement, cela compte dans les limites du forfait (pour Plus, la limite
sur 5 heures est partagée avec les autres apps) ; Terra ou Luna consomment moins
que Sol. Avec une clé API, c'est facturé au jeton. Un lot de 10
pages répond en quelques secondes : la page lue est prête rapidement, le reste
du chapitre est analysé en avance.

## Limites

- Lecteurs qui dessinent les pages dans un `<canvas>` (sans `<img>`) : non pris
  en charge.
- Firefox : l'extension non signée se recharge à chaque redémarrage
  (`about:debugging`).
- Les pages sont envoyées au service choisi (OpenAI par défaut).

## Évaluer et essayer

```bash
# Un chapitre depuis son URL, sans navigateur (téléchargement + analyse + scènes)
cargo run --release -p keytomusic-core --example mood_chapter -- https://sushiscan.net/one-piece-chapitre-1180/
# L'API seule (sans son ni interface), pour essayer l'extension
cargo run --release -p keytomusic-core --example mood_server -- --port 8765
# Ambiances proposées comparées à celles d'un profil déjà étiqueté
cargo run --release -p keytomusic-core --example autotag_eval -- profil.json --chatgpt --no-names
```


```bash
OPENAI_API_KEY=… cargo run --release -p keytomusic-core --example mood_eval -- \
    chemin/vers/RealTest --chapters BL,OP/1 --model gpt-6.1-sol --out resultats.json
```

`--chatgpt` utilise le compte connecté dans l'app à la place d'une clé
(`KEYTOMUSIC_DATA_DIR` pointe vers le dossier de données de l'app si besoin).

Le jeu annoté RealTest (ancien dossier `manga-mood-ai/test-images/RealTest`)
reste dans l'historique git. Référence de l'ancien pipeline local (Qwen3-VL 4B +
règles) sur BL + OP/1-5 (380 pages) : 55 % de pages exactes, 76 % de bonne
famille, pour 44 % en répondant toujours « tension ».

## Tests

- `cargo test -p keytomusic-core mood` : prompt et lecture des réponses,
  ordonnancement, téléchargements et pages demandées à l'extension, cache,
  extension exportée, images et tuiles, ambiances des sons (migration,
  sélection, choix du son).
- `cargo test -p keytomusic-core --test mood_flow` : parcours complet (appels de
  l'extension → moteur → faux serveur OpenAI → événements de lecture).
- `cargo test -p keytomusic-core --test chatgpt_flow` : connexion ChatGPT complète
  contre un faux serveur OpenAI (retour navigateur, échange du code, jeton
  d'identité signé, modèles, requête Responses en flux, renouvellement,
  déconnexion).
