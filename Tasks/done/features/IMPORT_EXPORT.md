# Phase 6 - Import/Export

> **Statut:** ✅ COMPLÉTÉE
> **Date de complétion:** 2026-01-24

---

## 6.1 Module Import/Export Backend

- [x] **6.1.1** Créer `src-tauri/src/import_export/mod.rs`
  - [x] Définir la structure du module
  - [x] Exporter les fonctions export et import
  **✅ Complété** - Module avec export/import submodules et ExportMetadata struct

- [x] **6.1.2** Créer les structures pour les métadonnées
  - [x] Définir `ExportMetadata` struct
  - [x] Champs: version, exported_at, app_version, platform
  **✅ Complété** - Struct avec serde Serialize/Deserialize

## 6.2 Export de Profil

- [x] **6.2.1** Créer `src-tauri/src/import_export/export.rs`
  - [x] Implémenter `export_profile(profile_id, output_path) -> Result<()>`
  **✅ Complété**

- [x] **6.2.2** Implémenter la logique d'export
  - [x] Charger le profil depuis storage
  - [x] Créer le sous-dossier `sounds/` dans le ZIP
  - [x] Copier tous les fichiers audio vers `sounds/`
  - [x] Mettre à jour les chemins dans le profil (chemins relatifs)
  - [x] Gérer les sons YouTube (copier depuis cache)
  - [x] Sérialiser le profil modifié en JSON
  - [x] Écrire `profile.json`
  - [x] Créer les métadonnées
  - [x] Écrire `metadata.json`
  **✅ Complété** - Écrit directement dans le ZIP sans répertoire temporaire (plus efficace)

- [x] **6.2.3** Créer le fichier ZIP
  - [x] Utiliser la crate `zip` pour créer le .ktm
  - [x] Ajouter tous les fichiers (profile.json, metadata.json, sounds/*)
  - [x] Compresser avec Deflate
  - [x] Écrire le fichier final à output_path
  - [x] Gestion des noms de fichiers dupliqués (make_unique_filename)
  **✅ Complété**

- [x] **6.2.4** Gérer les erreurs d'export
  - [x] Profil non trouvé (via storage::load_profile)
  - [x] Fichier audio manquant (vérifié avant ajout au ZIP)
  - [x] Erreur d'écriture fichier/zip
  - [x] Retourner des erreurs claires
  **✅ Complété**

## 6.3 Import de Profil

- [x] **6.3.1** Créer `src-tauri/src/import_export/import.rs`
  - [x] Implémenter `import_profile(ktm_path) -> Result<ProfileId>`
  **✅ Complété**

- [x] **6.3.2** Implémenter la logique d'import
  - [x] Vérifier que le fichier .ktm existe
  - [x] Ouvrir et lire le ZIP
  - [x] Vérifier la présence de profile.json
  - [x] Charger et parser metadata.json (optionnel, pour compatibilité future)
  - [x] Charger et parser profile.json
  **✅ Complété**

- [x] **6.3.3** Gérer les IDs et noms
  - [x] Générer un nouvel UUID pour le profil
  - [x] Mettre à jour profile.id
  - [x] Ajouter "(Imported)" au nom du profil
  - [x] Générer de nouveaux UUIDs pour tous les sons (éviter conflits)
  - [x] Mettre à jour les références dans keyBindings
  **✅ Complété**

- [x] **6.3.4** Copier les fichiers audio
  - [x] Créer le dossier `imported_sounds/{new_profile_id}/`
  - [x] Extraire les fichiers depuis le ZIP
  - [x] Mettre à jour les chemins dans les sons (chemins absolus)
  - [x] Fallback: essayer chemin complet puis nom de fichier seul
  **✅ Complété**

- [x] **6.3.5** Finaliser l'import
  - [x] Mettre à jour les timestamps (createdAt, updatedAt)
  - [x] Sauvegarder le nouveau profil via storage
  - [x] Retourner le nouveau ProfileId
  **✅ Complété**

- [x] **6.3.6** Gérer les erreurs d'import
  - [x] Fichier .ktm invalide ou corrompu (ZIP parsing error)
  - [x] Fichiers manquants dans le ZIP
  - [x] Erreur de parsing JSON
  - [x] Erreur de copie fichiers
  - [x] Retourner des erreurs claires
  **✅ Complété**

## 6.4 Commandes Import/Export Tauri

- [x] **6.4.1** Ajouter les commandes dans `commands.rs`
  - [x] `export_profile(profile_id: String, output_path: String) -> Result<(), String>`
  - [x] `import_profile(ktm_path: String) -> Result<String, String>`
  **✅ Complété** - Async commands avec tokio::spawn_blocking

- [x] **6.4.2** Ajouter les commandes de file dialogs
  - [x] `pick_save_location(default_name: String) -> Result<Option<String>, String>`
    - [x] Utiliser rfd (Rust File Dialog) pour dialogs natifs
    - [x] Filtre pour fichier .ktm
    - [x] Nom par défaut: "ProfileName.ktm"
  - [x] `pick_ktm_file() -> Result<Option<String>, String>`
    - [x] File picker pour .ktm
    - [x] Retourner le chemin sélectionné (ou null si annulé)
  **✅ Complété** - Utilise la crate `rfd` pour les dialogs natifs cross-platform

- [x] **6.4.3** Enregistrer les commandes dans main.rs
  **✅ Complété** - 4 nouvelles commandes enregistrées dans invoke_handler

## 6.5 Intégration Frontend

- [x] **6.5.1** Bouton Export dans Settings
  - [x] Bouton "Export Profile" dans SettingsModal
  - [x] Flow: pick save location → export → success/error message
  - [x] Désactivé si aucun profil sélectionné
  - [x] Status messages (Choosing location... / Exporting... / Success / Error)
  **✅ Complété**

- [x] **6.5.2** Bouton Import dans Settings
  - [x] Bouton "Import Profile" dans SettingsModal
  - [x] Flow: pick .ktm file → import → reload profiles → select imported
  - [x] Recharger la liste des profils après import
  - [x] Sélectionner automatiquement le profil importé
  - [x] Status messages
  **✅ Complété**

## 6.6 Export UX Improvements

- [x] **6.6.1** Barre de progression Export
  - [x] Émettre des events `export_progress` depuis le backend (current, total, filename)
  - [x] Créer `ExportProgress.tsx` - barre de progression flottante (bottom-right)
  - [x] Créer `exportStore.ts` - store Zustand global pour l'état d'export
  - [x] Afficher le compteur (current/total) et le nom du fichier en cours
  **✅ Complété** - Progress callback dans export.rs, event Tauri, composant flottant

- [x] **6.6.2** Bouton annulation d'export
  - [x] Ajouter `EXPORT_CANCELLED: AtomicBool` static dans export.rs
  - [x] Vérifier le flag entre chaque fichier copié dans la boucle d'export
  - [x] Sur annulation: supprimer le fichier temp, le tracking file, retourner erreur
  - [x] Ajouter commande Tauri `cancel_export`
  - [x] Bouton "x" sur le composant ExportProgress
  - [x] Toast "Export cancelled" (info, pas error)
  **✅ Complété**

- [x] **6.6.3** Interception fermeture fenêtre pendant export
  - [x] Handler `onCloseRequested` avec confirmation dialog
  - [x] Pattern `forceCloseRef` pour éviter boucle infinie
  - [x] Ajouter permissions `core:window:allow-destroy` et `core:window:allow-close`
  - [x] Appeler `cleanupExportTemp()` avant fermeture confirmée
  **✅ Complété**

- [x] **6.6.4** Nettoyage fichiers temporaires orphelins
  - [x] Écrire le chemin temp dans `export_in_progress.txt` avant export
  - [x] Supprimer le tracking file après export réussi
  - [x] `cleanup_interrupted_export()` au démarrage de l'app
  - [x] Commande Tauri `cleanup_export_temp`
  **✅ Complété**
