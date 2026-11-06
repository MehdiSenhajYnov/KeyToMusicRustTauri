# KeyToMusic

Application desktop Tauri pour piloter une soundboard de lecture manga, avec :

- lecture audio multi-pistes et crossfade
- assignation de sons a des touches globales
- import/export de profils
- telechargement et recherche YouTube
- systeme de decouverte
- Mood AI : un modèle de vision (GPT ou tout service compatible OpenAI) découpe le chapitre lu dans le navigateur en scènes et lance la musique associée

## Etat actuel

Le coeur soundboard/audio est stable. Mood AI (dans `keytomusic-core`, partagé avec l'édition native Awra) repose sur :

- un modèle de vision OpenAI payé par l'abonnement ChatGPT (« Sign in with ChatGPT », `gpt-6.1-sol` par défaut), ou par une clé API sur tout service compatible OpenAI (OpenRouter, Ollama, LM Studio…)
- une API HTTP locale sur `127.0.0.1:8765` par défaut, démarrée avec l'app quand Mood AI est activé
- une extension navigateur intégrée à l'app (bouton « Install extension ») qui envoie les pages du chapitre lu et la page à l'écran
- un découpage du chapitre en scènes (10 pages par requête, 2 pages de contexte) : la musique change avec la scène, pas à chaque page

Voir [docs/MOOD_AI.md](docs/MOOD_AI.md).

## Stack

- Desktop: Tauri 2.x
- Frontend: React 18 + TypeScript + Zustand + Tailwind CSS
- Backend: Rust
- Audio: rodio + cpal + symphonia
- Mood AI: API compatible OpenAI (GPT par défaut)
- Extension: WebExtension MV3 compatible Chromium + Firefox (`crates/keytomusic-core/extension/`)

## Commandes

```bash
npm install
npm run dev
npm run tauri:dev
npm run build
npm run tauri:build
cargo fmt --all
cargo clippy --workspace --all-targets
cargo test --workspace
cargo run --release -p keytomusic-core --example mood_eval -- <dataset>
```

Reference complete des commandes utiles :
[docs/COMMANDS_REFERENCE.md](/home/mehdi/Dev/KeyToMusicRustTauri/docs/COMMANDS_REFERENCE.md)

## Structure

```text
src/                     Frontend React/TypeScript
src-tauri/src/           Backend Rust/Tauri (commandes)
crates/keytomusic-core/  Moteur partagé (audio, touches, YouTube, Mood AI…)
crates/keytomusic-core/extension/ Extension navigateur Mood AI
docs/                    Documentation canonique et audit
Tasks/                   Suivi des taches et historique d'implementation
resources/               Ressources statiques packagees
data/                    Donnees runtime generees localement
```

## Documentation

- [docs/MOOD_AI.md](docs/MOOD_AI.md) : fonctionnement de Mood AI
- [docs/COMMANDS_REFERENCE.md](/home/mehdi/Dev/KeyToMusicRustTauri/docs/COMMANDS_REFERENCE.md) : index rapide des commandes lancables du repo
- [crates/keytomusic-core/extension/README.md](crates/keytomusic-core/extension/README.md) : installation et fonctionnement de l'extension
- [docs/KeyToMusic_Technical_Specification.md](/home/mehdi/Dev/KeyToMusicRustTauri/docs/KeyToMusic_Technical_Specification.md) : specification technique large de l'app
- [docs/DOCUMENTATION_AUDIT.md](/home/mehdi/Dev/KeyToMusicRustTauri/docs/DOCUMENTATION_AUDIT.md) : statut de chaque famille documentaire
- [Tasks/README.md](/home/mehdi/Dev/KeyToMusicRustTauri/Tasks/README.md) : suivi des taches actives, livrees et post-dev
- [CLAUDE.md](/home/mehdi/Dev/KeyToMusicRustTauri/CLAUDE.md) : reference agent/outillage, pas la doc produit canonique

## Notes

- Ne pas committer `data/`, `src-tauri/target/`, `.env*` ni les binaires auto-telecharges.
- L'ancienne recherche Mood AI locale (`manga-mood-ai/`, llama.cpp, Qwen3-VL) a été retirée ; elle reste dans l'historique git.
