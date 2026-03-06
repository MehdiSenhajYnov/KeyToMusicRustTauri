# Phase 7.5 - Legacy Import

> **Statut:** ✅ COMPLÉTÉE
> **Date de complétion:** 2026-01-24

---

## 7.5.1 Backend - Commande de conversion

- [x] **7.5.1.1** Définir les structs de parsing du format legacy
  - [x] `LegacySave` avec champ `Sounds: Vec<LegacyKeyEntry>`
  - [x] `LegacyKeyEntry` avec `Key` (u32), `UserKeyChar` (String), `SoundInfos` (Vec)
  - [x] `LegacySoundInfo` avec `uniqueId`, `soundPath`, `soundName`, `soundMomentum`
  **✅ Complété** - Structs avec `#[derive(serde::Deserialize)]` et `#[allow(non_snake_case)]`

- [x] **7.5.1.2** Implémenter `vk_to_keycode()` pour convertir les codes VK Windows en KeyCode web
  - [x] 65-90 → KeyA-KeyZ
  - [x] 48-57 → Digit0-Digit9
  - [x] 112-123 → F1-F12
  - [x] OEM keys (186-222) → Semicolon, Equal, Comma, etc.
  - [x] Touches spéciales (Space, Enter)
  **✅ Complété** - Mapping complet dans `commands.rs`

- [x] **7.5.1.3** Implémenter la commande `pick_legacy_file`
  - [x] File picker filtré sur `.json`
  **✅ Complété** - Utilise `rfd::FileDialog` avec filtre "Legacy Save" (*.json)

- [x] **7.5.1.4** Implémenter la commande `import_legacy_save`
  - [x] Lire et parser le fichier JSON legacy
  - [x] Créer un profil avec UUID, timestamps, nom dérivé du fichier
  - [x] Créer un track "OST" par défaut
  - [x] Convertir chaque entrée: VK code → keyCode, SoundInfos → Sound + KeyBinding
  - [x] Normaliser les chemins (`/` → `\` sur Windows)
  - [x] Sauvegarder le profil via `storage::save_profile`
  - [x] Logger le résultat (nombre de sons, bindings)
  **✅ Complété** - Conversion complète avec gestion des clés inconnues (skip avec warning)

- [x] **7.5.1.5** Enregistrer les commandes dans `main.rs`
  **✅ Complété** - `pick_legacy_file` et `import_legacy_save` dans `invoke_handler`

## 7.5.2 Frontend - Wrapper et UI

- [x] **7.5.2.1** Ajouter les fonctions dans `tauriCommands.ts`
  - [x] `pickLegacyFile(): Promise<string | null>`
  - [x] `importLegacySave(path: string): Promise<Profile>`
  **✅ Complété**

- [x] **7.5.2.2** Ajouter le bouton "Import Legacy Save" dans `SettingsModal.tsx`
  - [x] Bouton stylé en jaune (distinctif par rapport à l'import standard)
  - [x] Flow: pick file → convert → loadProfiles → loadProfile
  - [x] Affichage du status (converting, success, error) via `importStatus`
  **✅ Complété** - Bouton intégré dans la section Import/Export
