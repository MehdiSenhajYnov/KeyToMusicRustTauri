# Phase 6.5 - Concurrent YouTube Downloads & Key Cycling

> **Statut:** ✅ COMPLÉTÉE
> **Date de complétion:** 2026-01-24

---

## 6.7 Téléchargements YouTube Concurrents

- [x] **6.7.1** Backend: Ajouter `download_id` au command `add_sound_from_youtube`
  - [x] Paramètre `download_id: String` dans la signature
  - [x] Inclure `downloadId` dans le payload de l'event `youtube_download_progress`
  **✅ Complété**

- [x] **6.7.2** Frontend: Remplacer l'état single-download par multi-download
  - [x] Remplacer `isDownloading`/`downloadStatus`/`downloadProgress` par `activeDownloads` Map
  - [x] Chaque download trackée avec son propre ID, URL, status, progress
  - [x] URL input reste actif pendant les téléchargements (jamais disabled)
  - [x] Bouton Download uniquement disabled si URL vide
  - [x] Chaque download complété s'ajoute à la liste des fichiers
  **✅ Complété**

- [x] **6.7.3** Frontend: Affichage progression individuelle
  - [x] Barre de progression par téléchargement actif
  - [x] Spinner et status text par download
  - [x] Downloads retirés de la Map une fois terminés (succès ou erreur)
  **✅ Complété**

- [x] **6.7.4** Mise à jour `tauriCommands.ts`
  - [x] `addSoundFromYoutube(url, downloadId)` - nouveau paramètre downloadId
  **✅ Complété**

## 6.8 Key Cycling pour Assignation Multi-Sons

- [x] **6.8.1** Supprimer la limitation de longueur du champ keys
  - [x] Retirer `.slice(0, files.length)` dans `handleKeyInput`
  - [x] L'utilisateur peut taper moins de touches que de fichiers
  **✅ Complété**

- [x] **6.8.2** Affichage cycling dans la liste des fichiers
  - [x] Indicateur de touche par fichier utilise `keysInput[i % keysInput.length]`
  - [x] Reflète le cycling en temps réel pendant la saisie
  **✅ Complété**

- [x] **6.8.3** Logique de submit avec cycling (déjà implémentée)
  - [x] `keyCodes[i % keyCodes.length]` regroupe les sons par touche
  - [x] Un seul caractère "a" → tous les sons sur la même touche
  - [x] "ab" avec 5 sons → a,b,a,b,a
  **✅ Complété**
