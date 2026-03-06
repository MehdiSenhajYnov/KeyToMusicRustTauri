# Phase 11 - Build & Release (Bonus)

> **Statut:** ⏳ À FAIRE

---

## 11.1 Préparation du Build

- [ ] **11.1.1** Configurer les icônes
  - [ ] Créer toutes les tailles d'icônes requises
  - [ ] Optimiser les icônes

- [ ] **11.1.2** Configurer le bundle
  - [ ] Vérifier tauri.conf.json (identifier, version, etc.)
  - [ ] Configurer les targets (Windows, macOS, Linux)
  - [ ] Configurer les ressources à inclure

- [ ] **11.1.3** Optimiser le build
  - [ ] Build en mode release
  - [ ] Vérifier la taille du bundle
  - [ ] Strip les symboles de debug si nécessaire

## 11.2 Build par Plateforme

- [ ] **11.2.1** Build Windows
  - [ ] `npm run tauri build`
  - [ ] Vérifier le .exe et l'installeur .msi
  - [ ] Tester l'installation

- [ ] **11.2.2** Build macOS
  - [ ] `npm run tauri build`
  - [ ] Vérifier le .app et le .dmg
  - [ ] Signer l'app (si certificat disponible)
  - [ ] Tester l'installation

- [ ] **11.2.3** Build Linux
  - [ ] `npm run tauri build`
  - [ ] Vérifier le .deb et .AppImage
  - [ ] Tester l'installation sur Ubuntu

## 11.3 Documentation de Release

- [ ] **11.3.1** Changelog
  - [ ] Lister toutes les fonctionnalités
  - [ ] Lister les bugs connus (si applicable)

- [ ] **11.3.2** Instructions d'installation
  - [ ] Documenter l'installation pour chaque plateforme
  - [ ] Documenter l'installation de yt-dlp

- [ ] **11.3.3** Licence
  - [ ] Choisir une licence (MIT, GPL, etc.)
  - [ ] Ajouter LICENSE file

## 11.4 Distribution

- [ ] **11.4.1** Hébergement des releases
  - [ ] GitHub Releases
  - [ ] Uploader les binaires pour chaque plateforme

- [ ] **11.4.2** Auto-update (optionnel)
  - [ ] Configurer Tauri updater
  - [ ] Héberger le fichier de métadonnées d'update
