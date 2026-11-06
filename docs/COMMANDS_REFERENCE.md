# Command Reference

Reference rapide des commandes utiles du repo.  
Si une future conversation demande "comment lancer X ?", commencer ici.

## Setup

```bash
npm install
```

Installe les dependances frontend et la CLI Tauri.

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

## Évaluation Mood AI

```bash
OPENAI_API_KEY=sk-… cargo run --release -p keytomusic-core --example mood_eval -- \
    chemin/vers/RealTest --chapters BL,OP/1 --model gpt-6.1-sol --detail high --out resultats.json
```

Passe chaque chapitre du jeu annoté (un dossier par chapitre + `ground_truth.json`)
dans le même découpage en scènes que l'app, puis affiche la précision par page,
par famille d'ambiance, la référence « toujours tension » et le nombre de
changements de musique. `--base-url` permet de viser un autre service compatible
OpenAI (Ollama : `http://localhost:11434/v1`). Le jeu RealTest n'est plus dans le
dépôt : il reste dans l'historique git (`git show <commit>:manga-mood-ai/...`).
`--chatgpt` utilise le compte ChatGPT connecté dans l'app à la place d'une clé.

## Recommandations rapides

- Pour travailler sur l'app : `npm run tauri:dev`
- Pour juste voir le frontend web : `npm run dev`
- Pour mesurer Mood AI sur un jeu annoté : `cargo run --release -p keytomusic-core --example mood_eval -- <dossier>`
