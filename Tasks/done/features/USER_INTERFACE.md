# Phase 4 - Interface Utilisateur (React)

> **Statut:** ✅ COMPLÉTÉE
> **Date de complétion:** 2026-01-23

---

## 4.1 Types TypeScript

- [x] **4.1.1** Créer `src/types/index.ts`
  - [x] Définir les type aliases (SoundId, TrackId, ProfileId, KeyCode)
  - [x] Définir le type `SoundSource` (union type)
  - [x] Définir le type `LoopMode` (union type)
  - [x] Définir l'interface `Sound`
  - [x] Définir l'interface `KeyBinding`
  - [x] Définir l'interface `Track`
  - [x] Définir l'interface `Profile`
  - [x] Définir l'interface `AppConfig`
  - [x] Définir l'interface `NowPlayingState`
  - [x] Définir le type `BackendEvent` (union type pour tous les events)

## 4.2 Design System

- [x] **4.2.1** Configurer Tailwind avec les couleurs custom
  - [x] Éditer `tailwind.config.js` pour ajouter les couleurs du design system
  - [x] Définir bg-primary, bg-secondary, bg-tertiary, bg-hover
  - [x] Définir text-primary, text-secondary, text-muted
  - [x] Définir accent-primary, accent-secondary, success, warning, error
  - [x] Définir border-color, border-focus

- [x] **4.2.2** Créer les variables CSS dans `src/index.css`
  - [x] Définir toutes les couleurs en variables CSS
  - [x] Définir les tailles de police
  - [x] Définir la font-family
  - [x] Ajouter les styles de base pour le dark theme

## 4.3 Stores Zustand

- [x] **4.3.1** Créer `src/stores/audioStore.ts`
  - [x] Définir l'interface du state (tracks, sounds, nowPlaying)
  - [x] Créer le store avec zustand
  - [x] Implémenter les actions: setTracks, addTrack, removeTrack, updateTrack
  - [x] Implémenter les actions: setSounds, addSound, removeSound, updateSound
  - [x] Implémenter les actions: setNowPlaying, clearNowPlaying

- [x] **4.3.2** Créer `src/stores/profileStore.ts`
  - [x] Définir l'interface du state (profiles, currentProfile, keyBindings)
  - [x] Créer le store avec zustand
  - [x] Implémenter les actions: setProfiles, addProfile, removeProfile
  - [x] Implémenter les actions: setCurrentProfile, loadProfile
  - [x] Implémenter les actions: setKeyBindings, addKeyBinding, updateKeyBinding, removeKeyBinding

- [x] **4.3.3** Créer `src/stores/settingsStore.ts`
  - [x] Définir l'interface du state (config: AppConfig)
  - [x] Créer le store avec zustand
  - [x] Implémenter les actions: setConfig, updateConfig
  - [x] Implémenter les actions spécifiques: setMasterVolume, toggleAutoMomentum, toggleKeyDetection
  - [x] Implémenter: setMasterStopShortcut, setCrossfadeDuration, setKeyCooldown

## 4.4 Hooks Custom

- [x] **4.4.1** Créer `src/hooks/useKeyDetection.ts`
  - [x] Écouter l'event `key_pressed` via Tauri
  - [x] Mettre à jour le state quand une touche est détectée
  - [x] Déclencher les actions appropriées (highlight UI, etc.)
  - [x] Cleanup des listeners au unmount

- [x] **4.4.2** Créer `src/hooks/useAudioEngine.ts`
  - [x] Écouter les events `sound_started`, `sound_ended`, `playback_progress`
  - [x] Mettre à jour le state audioStore avec les infos de lecture
  - [x] Gérer le nowPlaying state
  - [x] Cleanup des listeners

- [x] **4.4.3** Créer `src/hooks/useTauriCommand.ts`
  - [x] Créer un hook générique pour invoquer des commandes Tauri
  - [x] Gérer le loading state
  - [x] Gérer les erreurs
  - [x] Retourner {execute, loading, error, data}

- [x] **4.4.4** Créer `src/hooks/useTextInputFocus.ts`
  - [x] Détecter le focus sur les input/textarea
  - [x] Désactiver la détection des touches via `invoke('set_key_detection', {enabled: false})`
  - [x] Réactiver la détection au blur
  - [x] Utiliser focusin/focusout events

## 4.5 Utils

- [x] **4.5.1** Créer `src/utils/fileHelpers.ts`
  - [x] Implémenter `formatDuration(seconds: number) -> string` (MM:SS)
  - [x] Implémenter `formatFileSize(bytes: number) -> string` (KB, MB, GB)
  - [x] Implémenter `getFileExtension(path: string) -> string`
  - [x] Implémenter `isAudioFile(path: string) -> boolean`

- [x] **4.5.2** Créer `src/utils/keyMapping.ts`
  - [x] Implémenter `keyCodeToDisplay(code: string) -> string` (KeyA → "A")
  - [x] Implémenter `parseKeyCombination(keys: string) -> string[]` ("adgk" → ["KeyA", "KeyD", "KeyG", "KeyK"])
  - [x] Implémenter `isValidKeyCode(code: string) -> boolean`

## 4.6 Layout Components

- [x] **4.6.1** Créer `src/components/Layout/Header.tsx`
  - [x] Props: masterVolume, onVolumeChange, onSettingsClick, onMinimize, onClose
  - [x] Afficher le logo et le nom "KeyToMusic"
  - [x] Créer le slider de volume master (horizontal, toujours visible)
  - [x] Créer le bouton paramètres (icône gear)
  - [x] Créer les boutons de fenêtre (minimize, close)
  - [x] Appeler les handlers appropriés
  - [x] Styling avec Tailwind (dark theme)

- [x] **4.6.2** Créer `src/components/Layout/Sidebar.tsx`
  - [x] Diviser en trois sections: Profiles, Controls, NowPlaying
  - [x] Créer la structure de layout (vertical flex)
  - [x] Styling avec Tailwind

- [x] **4.6.3** Créer `src/components/Layout/MainContent.tsx`
  - [x] Créer la zone principale pour afficher le contenu
  - [x] Diviser en deux sections: Track View (haut) et Sound Details (bas)
  - [x] Gérer le responsive layout
  - [x] Styling avec Tailwind

- [x] **4.6.4** Créer `src/App.tsx` avec le layout complet
  - [x] Importer tous les composants de layout
  - [x] Assembler Header + Sidebar + MainContent
  - [x] Utiliser les stores pour passer les props
  - [x] Gérer le routing/state de l'app si nécessaire
  - [x] Appliquer les styles globaux (min-width: 800px, min-height: 600px)

## 4.7 Profile Components

- [x] **4.7.1** Créer `src/components/Profiles/ProfileSelector.tsx`
  - [x] Props: profiles, currentProfileId, onSelect
  - [x] Afficher la liste des profils
  - [x] Highlight le profil actif
  - [x] Gérer le click pour sélectionner
  - [x] Styling avec Tailwind (liste verticale avec hover)

- [x] **4.7.2** Créer `src/components/Profiles/ProfileManager.tsx`
  - [x] Props: profiles, onProfileCreate, onProfileDelete, onProfileRename, onProfileExport, onProfileImport
  - [x] Bouton "+ New Profile" qui ouvre un modal
  - [x] Menu contextuel (click droit) pour rename/delete/export
  - [x] Appeler les commandes Tauri appropriées
  - [x] Gérer les confirmations (modal de confirmation pour delete)

- [x] **4.7.3** Créer le modal de création de profil
  - [x] Input pour le nom du profil
  - [x] Validation (nom non vide, max 50 caractères)
  - [x] Boutons Cancel/Create
  - [x] Appeler `invoke('create_profile', {name})`
  - [x] Fermer le modal et mettre à jour le state

- [x] **4.7.4** Créer le modal de rename de profil
  - [x] Input pré-rempli avec le nom actuel
  - [x] Validation
  - [x] Boutons Cancel/Save
  - [x] Appeler `invoke('save_profile')` avec le profil modifié

## 4.8 Controls Components

- [x] **4.8.1** Créer `src/components/Controls/MasterVolume.tsx`
  - [x] Props: volume, onChange
  - [x] Slider vertical ou horizontal
  - [x] Afficher le % à côté
  - [x] Appeler onChange qui invoque `invoke('set_master_volume')`
  - [x] Styling avec accent colors

- [x] **4.8.2** Créer `src/components/Controls/GlobalToggles.tsx`
  - [x] Props: autoMomentum, keyDetectionEnabled, onToggleAutoMomentum, onToggleKeyDetection
  - [x] Toggle switch pour Auto-Momentum avec label
  - [x] Toggle switch pour Key Detection avec label
  - [x] Indicateur visuel ON/OFF (couleur, icône)
  - [x] Appeler les handlers qui invoquent `update_config`

- [x] **4.8.3** Créer `src/components/Controls/MasterStopButton.tsx`
  - [x] Props: onClick
  - [x] Gros bouton rouge "Master Stop"
  - [x] Icône stop
  - [x] Au click: invoke('stop_all_sounds')
  - [x] Animation/feedback visuel au click

- [x] **4.8.4** Assembler dans Sidebar
  - [x] Créer une section "Controls" dans Sidebar
  - [x] Inclure GlobalToggles et MasterStopButton
  - [x] Organiser verticalement avec espacement

## 4.9 Now Playing Component

- [x] **4.9.1** Créer `src/components/Controls/NowPlaying.tsx`
  - [x] Props: nowPlayingState (peut être null)
  - [x] Afficher le nom de la piste
  - [x] Afficher le nom du son en cours
  - [x] Afficher la barre de progression (visual progress bar)
  - [x] Afficher currentTime / duration (formaté en MM:SS)
  - [x] Gérer le cas où rien ne joue (afficher un message ou vide)
  - [x] Styling compact pour la sidebar

- [x] **4.9.2** Connecter au store
  - [x] Lire le nowPlaying state depuis audioStore
  - [x] Mettre à jour en temps réel avec les events playback_progress

## 4.10 Track Components

- [x] **4.10.1** Créer `src/components/Tracks/TrackList.tsx`
  - [x] Props: tracks, selectedTrackId, onSelectTrack
  - [x] Afficher un dropdown pour sélectionner la piste
  - [x] Lister les tracks avec leur nom
  - [x] Highlight la piste sélectionnée
  - [x] Option "+ New Track" dans le dropdown

- [x] **4.10.2** Créer `src/components/Tracks/TrackItem.tsx`
  - [x] Props: track, isSelected, onSelect, onVolumeChange, onRename, onDelete
  - [x] Afficher le nom de la piste
  - [x] Slider de volume de la piste
  - [x] Icône/bouton pour rename
  - [x] Icône/bouton pour delete
  - [x] Styling avec hover states

- [x] **4.10.3** Créer `src/components/Tracks/TrackVolumeSlider.tsx`
  - [x] Props: volume, onChange
  - [x] Slider horizontal
  - [x] Afficher le % ou valeur
  - [x] Appeler onChange qui invoque `set_track_volume`

- [x] **4.10.4** Gérer la création de tracks
  - [x] Modal pour créer une nouvelle piste
  - [x] Input pour le nom
  - [x] Validation (nom non vide, limite de 20 pistes)
  - [x] Générer un UUID pour la nouvelle piste
  - [x] Ajouter au profil et sauvegarder

- [x] **4.10.5** Gérer la suppression de tracks
  - [x] Modal de confirmation
  - [x] Vérifier si des sons sont assignés à cette piste
  - [x] Avertir l'utilisateur si des sons seront affectés
  - [x] Retirer du profil et sauvegarder

## 4.11 Key Assignment Components

- [x] **4.11.1** Créer `src/components/Keys/KeyGrid.tsx`
  - [x] Props: keyBindings, sounds, selectedKey, onKeySelect
  - [x] Afficher une grille de cartes (flexbox grid)
  - [x] Chaque carte représente un key binding
  - [x] Afficher: [Touche] + nom du premier son + "(+N)" si plusieurs sons
  - [x] Highlight la carte si le son est en cours de lecture (depuis nowPlaying)
  - [x] Highlight la carte si elle est sélectionnée
  - [x] Au click sur une carte: onKeySelect(keyCode)
  - [x] Styling avec Tailwind (cards avec border, hover, active states)

- [x] **4.11.2** Créer `src/components/Keys/KeyItem.tsx`
  - [x] Props: keyBinding, sound, isPlaying, isSelected, onClick
  - [x] Afficher le key code (formaté: "KeyA" → "A")
  - [x] Afficher le nom du son
  - [x] Indicateur "(+N)" si plusieurs sons
  - [x] Indicateur de lecture (icône animée si isPlaying)
  - [x] Styling avec différentes couleurs pour différents états

- [x] **4.11.3** Créer le bouton "+ Add Sound"
  - [x] Bouton en bas de la grille
  - [x] Au click: ouvrir le modal d'ajout de son
  - [x] Styling avec accent color

## 4.12 Sound Detail Components

- [x] **4.12.1** Créer `src/components/Sounds/SoundList.tsx`
  - [x] Props: sounds (sons assignés à la touche sélectionnée)
  - [x] Afficher une liste des sons (vertical)
  - [x] Pour chaque son: utiliser SoundItem component
  - [x] Message si aucun son ("No sounds assigned")

- [x] **4.12.2** Créer `src/components/Sounds/SoundItem.tsx`
  - [x] Props: sound, onEdit, onRemove
  - [x] Afficher le nom du son
  - [x] Afficher le momentum (formaté en MM:SS)
  - [x] Afficher le volume individuel (%)
  - [x] Afficher la durée totale (formaté)
  - [x] Afficher la source (Local ou YouTube avec icône)
  - [x] Bouton "Edit" qui ouvre le modal de settings
  - [x] Bouton "Remove" qui demande confirmation
  - [x] Styling en card avec hover

- [x] **4.12.3** Créer `src/components/Sounds/SoundSettings.tsx` (modal)
  - [x] Props: sound, onSave, onCancel
  - [x] Input pour le nom
  - [x] Input pour le momentum (number, décimales autorisées)
  - [x] Slider pour le volume individuel
  - [x] Preview du fichier (si possible)
  - [x] Boutons Cancel/Save
  - [x] Validation
  - [x] Appeler onSave qui invoque `update_sound`

- [x] **4.12.4** Créer le sélecteur de Loop Mode
  - [x] Props: loopMode, onChange
  - [x] Dropdown ou boutons radio
  - [x] Options: Off, Random, Single, Sequential
  - [x] Descriptions pour chaque mode
  - [x] Appeler onChange qui met à jour le keyBinding dans le profil

- [x] **4.12.5** Assembler le panneau Sound Details
  - [x] Afficher seulement si une touche est sélectionnée
  - [x] Header: "Sounds for key [X]"
  - [x] SoundList
  - [x] Loop Mode selector
  - [x] Bouton "+ Add Sound to Key"

## 4.13 Add Sound Modal

- [x] **4.13.1** Créer `src/components/Sounds/AddSoundModal.tsx` (structure)
  - [x] Props: isOpen, onClose, existingTracks, onAdd
  - [x] Modal overlay avec backdrop
  - [x] Fermeture sur ESC ou click outside

- [x] **4.13.2** Étape 1: Choix de la source
  - [x] Boutons radio ou tabs: "Local File" / "YouTube URL"
  - [x] State pour tracker la source choisie

- [x] **4.13.3** Étape 2a: Si Local File
  - [x] Drag & Drop zone
  - [x] Highlight au drag over
  - [x] Bouton "Browse" qui ouvre le file picker
  - [x] Appeler `invoke('pick_audio_files')` ou utiliser input[type="file"]
  - [x] Validation du format (MP3, WAV, OGG, FLAC)
  - [x] Afficher les fichiers sélectionnés
  - [x] Support multi-fichiers

- [x] **4.13.4** Étape 2b: Si YouTube URL
  - [x] Input pour l'URL YouTube
  - [x] Validation de l'URL (format youtube.com/watch ou youtu.be)
  - [x] Bouton "Download"
  - [x] Appeler `invoke('add_sound_from_youtube', {url})`
  - [x] Écouter les events `download_progress`, `download_complete`, `download_error`
  - [x] Afficher une progress bar pendant le téléchargement
  - [x] Afficher le titre une fois téléchargé
  - [x] Gérer les erreurs (afficher message d'erreur)

- [x] **4.13.5** Étape 3: Configuration
  - [x] Input pour les touches à assigner (texte: "adgk")
  - [x] Parser les touches entrées (voir utils/keyMapping)
  - [x] Validation (touches valides, non déjà assignées au Master Stop)
  - [x] Dropdown pour choisir la piste (existantes + "New Track")
  - [x] Si "New Track": input pour le nom de la nouvelle piste
  - [x] Input number pour le momentum (secondes, décimales ok)
  - [x] Slider pour le volume individuel (0-100%)
  - [x] Dropdown pour le Loop Mode (défaut: "off")

- [x] **4.13.6** Étape 4: Ajout
  - [x] Bouton "Add" en bas du modal
  - [x] Validation de tous les champs
  - [x] Si plusieurs fichiers ET plusieurs touches: assignation cyclique
    - [x] Fichier 1 → Touche 1, Fichier 2 → Touche 2, etc.
    - [x] Si plus de fichiers que de touches: cycler les touches
  - [x] Créer les sons avec UUID
  - [x] Appeler `invoke('get_audio_duration', {path})` pour chaque son
  - [x] Créer les KeyBindings
  - [x] Créer la Track si nouvelle
  - [x] Mettre à jour le profil
  - [x] Appeler `invoke('save_profile', {profile})`
  - [x] Fermer le modal
  - [x] Afficher un toast de succès

## 4.14 Settings Modal

- [x] **4.14.1** Créer `src/components/Settings/SettingsModal.tsx`
  - [x] Props: isOpen, onClose, config, onConfigUpdate
  - [x] Modal overlay
  - [x] Titre "Settings"

- [x] **4.14.2** Section: Master Stop Shortcut
  - [x] Afficher la combinaison actuelle (formaté: "Ctrl+Shift+S")
  - [x] Bouton "Change"
  - [x] Au click: mode capture
  - [x] Afficher "Press the key combination..."
  - [x] Capturer les touches pressées (via event listeners sur window)
  - [x] Afficher les touches capturées en temps réel
  - [x] Bouton "Save" pour confirmer
  - [x] Validation (au moins 2 touches, combinaison valide)
  - [x] Appeler `invoke('set_master_stop_shortcut', {keys})`

- [x] **4.14.3** Section: Crossfade Duration
  - [x] Slider (100ms à 2000ms)
  - [x] Afficher la valeur actuelle en ms
  - [x] Appeler `update_config` au changement

- [x] **4.14.4** Section: Key Cooldown
  - [x] Slider (500ms à 5000ms)
  - [x] Afficher la valeur actuelle en ms
  - [x] Appeler `update_config` au changement

- [x] **4.14.5** Section: Import/Export
  - [x] Bouton "Export Current Profile"
  - [x] Au click: ouvrir save dialog via `invoke('pick_save_location')`
  - [x] Appeler `invoke('export_profile', {profileId, outputPath})`
  - [x] Afficher un toast de succès/erreur
  - [x] Bouton "Import Profile"
  - [x] Au click: ouvrir file picker (filtre .ktm)
  - [x] Appeler `invoke('import_profile', {ktmPath})`
  - [x] Afficher un toast et recharger la liste des profils

- [x] **4.14.6** Section: About
  - [x] Afficher le nom de l'app "KeyToMusic"
  - [x] Afficher la version (lire depuis package.json ou Tauri config)
  - [x] Liens vers la documentation ou GitHub
  - [x] Informations de licence si applicable

- [x] **4.14.7** Boutons du modal
  - [x] Bouton "Close" en bas
  - [x] Fermer sur ESC

## 4.15 Error Modals

- [x] **4.15.1** Créer `src/components/Modals/FileNotFoundModal.tsx`
  - [x] Props: soundName, expectedPath, onUpdatePath, onRemoveSound, onCancel
  - [x] Afficher le message: "Le fichier audio n'a pas été trouvé"
  - [x] Afficher le nom du son et le chemin attendu
  - [x] Bouton "Update Path"
    - [x] Ouvrir file picker
    - [x] Appeler onUpdatePath(newPath)
    - [x] Mettre à jour le sound dans le profil
    - [x] Sauvegarder
  - [x] Bouton "Remove Sound"
    - [x] Confirmation
    - [x] Appeler onRemoveSound()
    - [x] Retirer du profil et sauvegarder
  - [x] Bouton "Cancel"
    - [x] Fermer le modal sans action

- [x] **4.15.2** Créer `src/components/Modals/ErrorModal.tsx` (générique)
  - [x] Props: title, message, onClose
  - [x] Afficher le titre et message d'erreur
  - [x] Icône d'erreur
  - [x] Bouton "OK" pour fermer
  - [x] Styling avec couleur error

- [x] **4.15.3** Gérer l'affichage des modals d'erreur
  - [x] Écouter l'event `sound_not_found`
  - [x] Afficher FileNotFoundModal avec les infos du son
  - [x] Écouter l'event `download_error`
  - [x] Afficher ErrorModal avec le message

## 4.16 Notifications/Toasts

- [x] **4.16.1** Créer un système de notifications toast
  - [x] Créer `src/components/Toast/ToastContainer.tsx`
  - [x] Gérer un state de toasts (array)
  - [x] Afficher les toasts en overlay (coin haut-droit ou bas-droit)
  - [x] Auto-dismiss après 3-5 secondes
  - [x] Types: success, error, info, warning

- [x] **4.16.2** Créer `src/components/Toast/Toast.tsx`
  - [x] Props: type, message, onClose
  - [x] Icône selon le type
  - [x] Message
  - [x] Bouton close (X)
  - [x] Animation d'entrée/sortie
  - [x] Styling avec les couleurs appropriées

- [x] **4.16.3** Créer un hook `useToast`
  - [x] Exposer une fonction `showToast(message, type)`
  - [x] Ajouter le toast au state
  - [x] Gérer l'auto-dismiss
  - [x] Retourner la fonction showToast

- [x] **4.16.4** Intégrer les toasts dans l'app
  - [x] Ajouter ToastContainer dans App.tsx
  - [x] Utiliser showToast pour les succès (profil créé, son ajouté, etc.)
  - [x] Utiliser showToast pour les erreurs
  - [x] Utiliser showToast pour les infos (téléchargement en cours, etc.)

## 4.17 Intégration des Commandes Tauri

- [x] **4.17.1** Créer les wrappers de commandes dans `src/utils/tauriCommands.ts`
  - [x] Wrapper pour toutes les commandes config
  - [x] Wrapper pour toutes les commandes profile
  - [x] Wrapper pour toutes les commandes audio
  - [x] Wrapper pour toutes les commandes sounds
  - [x] Wrapper pour toutes les commandes keys
  - [x] Wrapper pour toutes les commandes import/export
  - [x] Typage TypeScript pour les paramètres et retours
  - [x] Gestion des erreurs avec try/catch

- [x] **4.17.2** Utiliser les wrappers dans les composants
  - [x] Remplacer les `invoke()` par les wrappers typés
  - [x] Gérer les erreurs de manière consistante
  - [x] Afficher les toasts appropriés

## 4.18 Event Listeners

- [x] **4.18.1** Créer `src/utils/tauriEvents.ts`
  - [x] Créer des fonctions pour écouter chaque type d'event
  - [x] `onSoundStarted(callback)`
  - [x] `onSoundEnded(callback)`
  - [x] `onPlaybackProgress(callback)`
  - [x] `onKeyPressed(callback)`
  - [x] `onDownloadProgress(callback)`
  - [x] `onDownloadComplete(callback)`
  - [x] `onDownloadError(callback)`
  - [x] `onSoundNotFound(callback)`
  - [x] Retourner les fonctions unlisten pour cleanup

- [x] **4.18.2** Utiliser les event listeners dans les composants
  - [x] Écouter dans useEffect
  - [x] Cleanup au unmount
  - [x] Mettre à jour les stores appropriés
