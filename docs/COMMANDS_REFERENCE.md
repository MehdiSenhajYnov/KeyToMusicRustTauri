# Command Reference

Reference rapide des commandes utiles du repo.  
Si une future conversation demande "comment lancer X ?" ou "comment relancer le benchmark ?", commencer ici.

## Setup

```bash
npm install
```

Installe les dependances frontend et la CLI Tauri.

```bash
bash scripts/install-realtest-benchmark-deps.sh
```

Installe `llama-server` et les modeles du benchmark RealTest (`Qwen3-VL-4B-Thinking`).

```bash
bash scripts/install-realtest-benchmark-deps.sh --gemma4-e4b
```

Ajoute aussi `Gemma 4 E4B` (Q4_K_M + `mmproj-F16`) pour les benchs `RealTest`.

```bash
bash scripts/install-realtest-benchmark-deps.sh --gemma4-e2b
```

Ajoute aussi `Gemma 4 E2B` (Q4_K_M + `mmproj-F16`) pour les benchs `RealTest`.

## App principale

```bash
npm run dev
```

Lance uniquement le frontend Vite sur `http://localhost:5173`.

```bash
npm run tauri:dev
```

Lance l'application desktop Tauri complete avec hot reload.

```bash
npm run build
```

Build web de production dans `dist/`.

```bash
npm run preview
```

Sert localement le build web `dist/`.

```bash
npm run tauri:build
```

Build desktop Tauri.

```bash
npm run tauri:build:rpm
```

Build le bundle RPM Linux.

## Tests frontend et parcours UI

```bash
npm test
```

Lance les tests Vitest (sauvegardes, profils, réglages, affectations et lecture).

```bash
npx playwright-core install chromium
npm run test:ui
```

Installe le navigateur de test, puis lance les parcours UI avec un backend Tauri simulé. Le script démarre et arrête son serveur Vite sur `127.0.0.1:5199`. Il ne modifie pas la bibliothèque réelle. Les captures sont écrites dans le dossier temporaire système `keytomusic-ui-smoke`, ou dans `KTM_SMOKE_ARTIFACTS` si défini.

Pour utiliser un Chromium déjà installé :

```bash
PLAYWRIGHT_CHROMIUM_EXECUTABLE_PATH=/chemin/vers/chromium npm run test:ui
```

Pour ajouter les régressions AZERTY (affichage initial, redémarrage et changement de disposition) :

```bash
KTM_TEST_AZERTY=1 npm run test:ui
```

Pour compiler l'application native de développement sans fabriquer d'installateur :

```bash
npm run tauri -- build --debug --no-bundle
```

Voir le [rapport d'audit du 6 septembre 2026](AUDIT_APPLICATION_2026-09-06.md) pour les corrections et les limites de validation.

## Checks backend

```bash
cargo check --manifest-path src-tauri/Cargo.toml
```

Verification de compilation Rust rapide.

```bash
cargo fmt --all
```

Formate le backend Rust.

```bash
cargo clippy --workspace --all-targets
```

Lint Rust.

```bash
cargo test --workspace
```

Lance les tests Rust classiques.

## RealTest viewer web

```bash
python3 scripts/serve-realtest-viewer.py
```

Lance le mini serveur web local du RealTest sur `127.0.0.1:8766`.
Ce port est separe du Mood API de KeyToMusic, qui reste sur `8765`.

Routes utiles :

- `http://127.0.0.1:8766/realtest` : hub
- `http://127.0.0.1:8766/compare` : comparateur
- `http://127.0.0.1:8766/ground-truth` : viewer du ground truth

Comparaison smoothing OST :

- la methode Production de base est `realtest_suite_t4b_wide5_selective_hold.json`
- son companion smoothing OST est `realtest_suite_t4b_wide5_selective_hold_soundtrack_commit.json`
- le ground truth `Reference` peut aussi etre compare a `Reference + OST commit`
- dans le catalogue du viewer, le bouton `Compare OST` compare directement le resultat page par page de base avec la variante `+ OST commit`

Options :

```bash
python3 scripts/serve-realtest-viewer.py --host 127.0.0.1 --port 8766
```

## Benchmark RealTest direct

Le benchmark RealTest ne se lance plus via `cargo test realtest_benchmark`.  
Le bon point d'entree est maintenant le mode CLI du binaire Tauri :

```bash
KEYTOMUSIC_RUN_REALTEST_BENCHMARK=1 \
REALTEST_FILTER=BL/2 \
REALTEST_EXPERIMENTS=t4b_wide5_selective_hold \
cargo run --manifest-path src-tauri/Cargo.toml
```

Description :

- `KEYTOMUSIC_RUN_REALTEST_BENCHMARK=1` : lance le mode benchmark CLI sans ouvrir l'app
- `REALTEST_FILTER=BL/2` : limite a une serie ou un chapitre (`BL`, `BL/2`, `OP/1`, etc.)
- `REALTEST_EXPERIMENTS=t4b_wide5_selective_hold` : methode Production actuelle

Smoke run court :

```bash
KEYTOMUSIC_RUN_REALTEST_BENCHMARK=1 \
REALTEST_FILTER=BL/2 \
REALTEST_PAGE_LIMIT=1 \
REALTEST_EXPERIMENTS=t4b_wide5_selective_hold \
cargo run --manifest-path src-tauri/Cargo.toml
```

Smoke run `Gemma 4` :

```bash
KEYTOMUSIC_RUN_REALTEST_BENCHMARK=1 \
REALTEST_FILTER=BL/1 \
REALTEST_PAGE_LIMIT=1 \
REALTEST_EXPERIMENTS=gemma4_e4b_wide5_selective_hold \
KEYTOMUSIC_LLAMA_GPU_LAYERS=24 \
cargo run --manifest-path src-tauri/Cargo.toml
```

```bash
KEYTOMUSIC_RUN_REALTEST_BENCHMARK=1 \
REALTEST_FILTER=BL/1 \
REALTEST_PAGE_LIMIT=1 \
REALTEST_EXPERIMENTS=gemma4_e2b_wide5_selective_hold \
KEYTOMUSIC_LLAMA_GPU_LAYERS=24 \
cargo run --manifest-path src-tauri/Cargo.toml
```

`Gemma 4` a ete valide localement avec `KEYTOMUSIC_LLAMA_GPU_LAYERS=24` sur RTX 4070 12 Go.
Si un autre projet local consomme deja le GPU, attendre qu'il finisse avant de lancer ce smoke run.

Mini showdown `Gemma 4` vs baseline courante :

```bash
KEYTOMUSIC_RUN_REALTEST_BENCHMARK=1 \
REALTEST_FILTER=BL/1 \
REALTEST_PAGE_LIMIT=3 \
REALTEST_EXPERIMENTS=gemma4_showdown \
KEYTOMUSIC_LLAMA_GPU_LAYERS=24 \
cargo run --manifest-path src-tauri/Cargo.toml
```

Si les poids ne sont pas dans `manga-mood-ai/models/`, tu peux aussi pointer des chemins custom :

```bash
REALTEST_MODEL_GEMMA4_E4B=/abs/path/gemma-4-E4B-it-Q4_K_M.gguf \
REALTEST_MMPROJ_GEMMA4_E4B=/abs/path/mmproj-F16.gguf \
KEYTOMUSIC_RUN_REALTEST_BENCHMARK=1 \
REALTEST_FILTER=BL/1 \
REALTEST_PAGE_LIMIT=1 \
REALTEST_EXPERIMENTS=gemma4_e4b_wide5_selective_hold \
KEYTOMUSIC_LLAMA_GPU_LAYERS=24 \
cargo run --manifest-path src-tauri/Cargo.toml
```

```bash
REALTEST_MODEL_GEMMA4_E2B=/abs/path/gemma-4-E2B-it-Q4_K_M.gguf \
REALTEST_MMPROJ_GEMMA4_E2B=/abs/path/mmproj-F16.gguf \
KEYTOMUSIC_RUN_REALTEST_BENCHMARK=1 \
REALTEST_FILTER=BL/1 \
REALTEST_PAGE_LIMIT=1 \
REALTEST_EXPERIMENTS=gemma4_e2b_wide5_selective_hold \
cargo run --manifest-path src-tauri/Cargo.toml
```

Regenerer uniquement le companion smoothing OST a partir du cumul Production deja present, sans rerun du modele :

```bash
KEYTOMUSIC_RUN_REALTEST_BENCHMARK=1 \
REALTEST_EXPERIMENTS=production_cache \
cargo run --manifest-path src-tauri/Cargo.toml
```

Sorties utiles :

- base Production : `manga-mood-ai/results/realtests/realtest_suite_t4b_wide5_selective_hold.json`
- companion smoothing OST : `manga-mood-ai/results/realtests/realtest_suite_t4b_wide5_selective_hold_soundtrack_commit.json`

## Benchmark Production resumable

Le runner principal pour le benchmark long terme est :

```bash
python3 scripts/run-realtest-prod-benchmark.py start
```

Il :

- reprend automatiquement a partir du cumul deja sauve
- checkpoint le resultat Production a la fin de chaque chapitre
- saute les images illisibles/corrompues
- permet un stop propre apres le chapitre en cours

Autres commandes :

```bash
python3 scripts/run-realtest-prod-benchmark.py status
python3 scripts/run-realtest-prod-benchmark.py stop
python3 scripts/run-realtest-prod-benchmark.py run
```

Description :

- `start` : lance le runner en arriere-plan
- `status` : affiche le dernier statut connu
- `stop` : demande un arret propre apres le chapitre en cours
- `run` : lance le runner au premier plan

Fichiers utiles du runner :

- cumul Production : `manga-mood-ai/results/realtests/realtest_suite_t4b_wide5_selective_hold.json`
- statut : `data/realtest-prod-benchmark/status.json`
- log : `data/realtest-prod-benchmark/runner.log`

Important :

- un arret brutal de la machine peut laisser un statut "running" stale, meme si le PID n'existe plus
- le cumul n'est mis a jour qu'a la fin de chaque chapitre
- si la machine s'arrete au milieu d'un chapitre, seul ce chapitre en cours est perdu

## Recommandations rapides

- Pour travailler sur l'app : `npm run tauri:dev`
- Pour juste voir le frontend web : `npm run dev`
- Pour ouvrir le viewer RealTest : `python3 scripts/serve-realtest-viewer.py`
- Pour lancer le benchmark Production long : `python3 scripts/run-realtest-prod-benchmark.py start`
- Pour voir si le benchmark tourne encore : `python3 scripts/run-realtest-prod-benchmark.py status`
