# KeyToMusic

Application desktop Tauri pour piloter une soundboard de lecture manga, avec :

- lecture audio multi-pistes et crossfade
- assignation de sons a des touches globales
- import/export de profils
- telechargement et recherche YouTube
- systeme de decouverte
- pipeline Manga Mood local avec extension navigateur + runtime VLM local

## Etat actuel

Le coeur soundboard/audio est stable.  
Le sous-systeme Manga Mood actif repose sur :

- un runtime local `llama-server` pilote par l'app Tauri
- une API HTTP locale sur `127.0.0.1:8765` par defaut
- une extension navigateur qui precharge les pages manga dans le navigateur, les capture en base64, et alimente le cache mood du chapitre
- un pipeline cible oriente lecture utilisateur, avec buffer analyse vise `X-10 .. X+20`

La source de verite pour ce flux est [docs/MANGA_MOOD_CURRENT_ARCHITECTURE.md](/home/mehdi/Dev/KeyToMusicRustTauri/docs/MANGA_MOOD_CURRENT_ARCHITECTURE.md).

## Stack

- Desktop: Tauri 2.x
- Frontend: React 18 + TypeScript + Zustand + Tailwind CSS
- Backend: Rust
- Audio: rodio + cpal + symphonia
- Mood AI: llama.cpp `llama-server` + `Qwen3-VL-4B-Thinking`
- Extension: WebExtension compatible Chromium + Firefox

## Commandes

```bash
npm install
npm run dev
npm run tauri:dev
npm run build
npm run tauri:build
cargo fmt --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
```

## Structure

```text
src/                     Frontend React/TypeScript
src-tauri/src/           Backend Rust/Tauri
WebExtension/manga-mood/ Extension navigateur Manga Mood
docs/                    Documentation canonique et audit
Tasks/                   Suivi des taches et historique d'implementation
manga-mood-ai/           Recherche, plans et benchmarks mood
resources/               Ressources statiques packagees
data/                    Donnees runtime generees localement
```

## Documentation

- [docs/MANGA_MOOD_CURRENT_ARCHITECTURE.md](/home/mehdi/Dev/KeyToMusicRustTauri/docs/MANGA_MOOD_CURRENT_ARCHITECTURE.md) : architecture produit actuelle du systeme Manga Mood
- [WebExtension/manga-mood/README.md](/home/mehdi/Dev/KeyToMusicRustTauri/WebExtension/manga-mood/README.md) : installation, workflow et debug de l'extension
- [docs/KeyToMusic_Technical_Specification.md](/home/mehdi/Dev/KeyToMusicRustTauri/docs/KeyToMusic_Technical_Specification.md) : specification technique large de l'app
- [docs/DOCUMENTATION_AUDIT.md](/home/mehdi/Dev/KeyToMusicRustTauri/docs/DOCUMENTATION_AUDIT.md) : statut de chaque famille documentaire
- [Tasks/README.md](/home/mehdi/Dev/KeyToMusicRustTauri/Tasks/README.md) : suivi des taches actives, livrees et post-dev
- [CLAUDE.md](/home/mehdi/Dev/KeyToMusicRustTauri/CLAUDE.md) : reference agent/outillage, pas la doc produit canonique

## Notes

- Ne pas committer `data/`, `src-tauri/target/`, `.env*` ni les binaires auto-telecharges.
- Les docs `manga-mood-ai/plans/`, `manga-mood-ai/research/` et `manga-mood-ai/results/` contiennent de la recherche, de l'historique et des artefacts de benchmark. Elles ne remplacent pas la doc produit canonique.
