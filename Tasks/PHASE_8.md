# Phase 8 - Nouvelles Features

> **Statut:** ✅ COMPLÉTÉE
> **Date de complétion:** 2026-01-25

Cette phase ajoute des fonctionnalités demandées pour améliorer l'UX sans alourdir l'interface.

---

## 8.1 Duplication de Profil ✅

- [x] **8.1.1** Ajouter la commande backend `duplicate_profile`
  - [x] Créer `src-tauri/src/storage/profile.rs::duplicate_profile(id: String, new_name: String)`
  - [x] Charger le profil source
  - [x] Générer un nouvel UUID pour le profil dupliqué
  - [x] Mettre à jour `createdAt` et `updatedAt`
  - [x] Copier tous les sons, tracks, et key bindings
  - [x] Sauvegarder le nouveau profil
  - [x] Retourner le profil dupliqué
  **✅ Complété** - Fonction `duplicate_profile` ajoutée dans `storage/profile.rs`

- [x] **8.1.2** Ajouter la commande Tauri `duplicate_profile`
  - [x] Créer dans `commands.rs`: `duplicate_profile(id: String, new_name: Option<String>) -> Result<Profile, String>`
  - [x] Si `new_name` est None, utiliser "{original_name} (Copy)"
  - [x] Enregistrer la commande dans `main.rs`
  **✅ Complété** - Commande Tauri créée et enregistrée

- [x] **8.1.3** Ajouter l'option dans le menu contextuel du ProfileSelector
  - [x] Ajouter "Duplicate" dans le menu (bouton ⎘ avant Delete)
  - [x] Appeler `duplicateProfile` du profileStore
  - [x] Rafraîchir la liste des profils après duplication
  - [x] Sélectionner automatiquement le profil dupliqué
  **✅ Complété** - Bouton "Duplicate" ajouté avec icône ⎘

- [x] **8.1.4** Ajouter `duplicateProfile` dans `profileStore.ts`
  - [x] Appeler `commands.duplicateProfile(id)`
  - [x] Ajouter le nouveau profil à la liste
  - [x] Sélectionner le nouveau profil
  **✅ Complété** - Fonction ajoutée au store et à tauriCommands.ts

## 8.2 Raccourcis Clavier Combinés (Modificateurs) ✅

Permettre l'utilisation de combinaisons comme Ctrl+A, Shift+F1, Alt+1 comme triggers de sons.

- [x] **8.2.1** Modifier le type `KeyBinding` pour supporter les modificateurs
  - [x] Utiliser une notation combinée dans `keyCode` (ex: "Ctrl+KeyA")
  - [x] Approche choisie: notation string combinée (plus simple, backward compatible)
  **✅ Complété** - Notation combinée "Ctrl+Shift+KeyA" utilisée

- [x] **8.2.2** Modifier le détecteur de touches backend (`detector.rs`)
  - [x] Lors d'un KeyPress, vérifier si des modificateurs sont maintenus
  - [x] Construire le code combiné (ex: si Ctrl+Shift maintenus et KeyA pressé → "Ctrl+Shift+KeyA")
  - [x] Émettre l'événement avec le code combiné
  - [x] Ne pas bloquer les touches modificateurs seules
  **✅ Complété** - Ordre: Ctrl > Shift > Alt > Key

- [x] **8.2.3** Modifier le frontend pour supporter les combinaisons
  - [x] Mettre à jour `useKeyDetection.ts` pour construire le code combiné
  - [x] Matcher d'abord le code combiné, puis fallback sur la touche de base
  - [x] Shift+X sur binding "X" applique le momentum (comportement existant préservé)
  **✅ Complété** - Logique de matching avec fallback implémentée

- [x] **8.2.4** Mettre à jour `keyMapping.ts` pour l'affichage
  - [x] Fonction `keyCodeToDisplay` mise à jour pour gérer "Ctrl+Shift+A"
  - [x] Fonctions `buildKeyCombo` et `parseKeyCombo` ajoutées
  - [x] Gérer l'ordre d'affichage (Ctrl avant Shift avant Alt avant la touche)
  **✅ Complété** - Affichage correct des combinaisons

- [x] **8.2.5** Mettre à jour les validations
  - [x] Fonction `checkKeyComboConflict` ajoutée
  - [x] Vérifie les conflits avec Ctrl+C/V/X/Z/Y/A/S/W/Q/N/T, Alt+F4
  - [x] Avertit pour Ctrl+chiffre (tabs) et Alt+lettre (menus Windows)
  **✅ Complété** - Validation des conflits système implémentée

- [x] **8.2.6** Validation étendue des raccourcis réservés
  - [x] Étendre `checkKeyComboConflict` → `checkShortcutConflicts(combo, config)`
  - [x] Bloquer les raccourcis app (Ctrl+Z, Ctrl+Y pour Undo/Redo)
  - [x] Bloquer les global shortcuts configurés par l'utilisateur
  - [x] Bloquer les raccourcis système (Ctrl+C/V/X/A/S/W/Q/N/T, Alt+F4)
  - [x] Warning (pas blocage) pour Ctrl+1-9 (tabs) et Alt+lettre (menus Windows)
  **✅ Complété** - `checkShortcutConflicts()` ajoutée dans keyMapping.ts

- [x] **8.2.7** Refonte UI AddSoundModal pour key assignment
  - [x] Créer composant `KeyCaptureSlot` réutilisable
  - [x] Remplacer l'input texte "aze" par une liste de slots de capture
  - [x] Chaque slot capture une combinaison de touches
  - [x] Bouton "+" pour ajouter un slot
  - [x] Bouton "×" pour supprimer un slot
  - [x] Preview du cycling en temps réel
  **✅ Complété** - `KeyCaptureSlot.tsx` créé, AddSoundModal refactoré

- [x] **8.2.8** Mise à jour KeyGrid et SoundDetails pour afficher les combinaisons
  - [x] Gérer les noms plus longs ("Ctrl+Shift+A" vs "A")
  - [x] Truncate avec max-width et tooltip au survol
  - [x] SoundDetails: capture avec support modifiers
  **✅ Complété** - KeyGrid et SoundDetails mis à jour

## 8.3 Système Undo/Redo ✅

Implémenter Ctrl+Z (Undo) et Ctrl+Y (Redo) pour les modifications de profil.

- [x] **8.3.1** Créer le store d'historique `historyStore.ts`
  - [x] Définir le type `HistoryEntry` (timestamp, action, previousState, newState)
  - [x] Stack `past: HistoryEntry[]` pour undo
  - [x] Stack `future: HistoryEntry[]` pour redo
  - [x] Limite de 50 entrées maximum
  - [x] Actions: `pushState(entry)`, `undo()`, `redo()`, `clear()`
  **✅ Complété** - Store d'historique complet créé

- [x] **8.3.2** Définir les actions annulables
  - [x] Suppression de son (`removeSound`)
  - [x] Suppression de binding (`removeKeyBinding`)
  - [x] Suppression de track (`removeTrack`)
  - [x] Modification de binding (loopMode, name, soundIds, trackId)
  - [x] Modification de son (volume, momentum, nom)
  - [x] Ajout de son/track/binding
  - [x] **Non annulable**: création de profil, suppression de profil, téléchargements YouTube
  **✅ Complété** - Actions annulables identifiées et filtrées

- [x] **8.3.3** Intégrer avec `profileStore.ts`
  - [x] Avant chaque action annulable, capturer l'état via `captureProfileState()`
  - [x] Après l'action, pusher l'entrée dans l'historique
  - [x] `undo()`: restaurer l'état précédent
  - [x] `redo()`: restaurer l'état suivant
  - [x] Clear history au changement de profil
  **✅ Complété** - Intégration complète avec profileStore

- [x] **8.3.4** Implémenter les raccourcis clavier
  - [x] Créer `useUndoRedo.ts` hook
  - [x] Ctrl+Z / Cmd+Z → undo
  - [x] Ctrl+Y / Cmd+Shift+Z → redo
  - [x] Désactiver quand un champ de texte est focus
  - [x] Feedback toast: "Undo: {action}" / "Redo: {action}"
  **✅ Complété** - Hook créé et intégré dans App.tsx

- [ ] **8.3.5** Indicateur visuel (optionnel)
  - [ ] Griser Undo si `past` est vide
  - [ ] Griser Redo si `future` est vide
  **⏳ Optionnel** - Non implémenté (UI non alourdie)

## 8.4 Multi-Key Chords ✅

Permettre des combinaisons de touches non-modifier pressées simultanément (comme un accord de piano).
Système inspiré des combos de jeux de combat (Street Fighter, Tekken).

> **Voir aussi**: `docs/PHASE_8_COMBINED_SHORTCUTS_PLAN.md` section 3 pour les détails complets.

**Principe : Arbre préfixe (Trie) + Trigger intelligent**
- Trigger immédiat si le combo actuel est une "feuille" (pas d'extensions possibles)
- Sinon attendre timer ou prochaine touche
- Latence 0ms pour les touches sans extensions possibles

**Exemple avec bindings A, A+Z, A+Z+E :**
```
A pressé → Extensions possibles (A+Z, A+Z+E) → Timer 30ms démarre
Z pressé → Extensions possibles (A+Z+E) → Timer continue
E pressé → Feuille (pas de A+Z+E+*) → TRIGGER IMMÉDIAT "A+Z+E"
```

- [x] **8.4.1** Implémenter la structure Trie (arbre préfixe)
  - [x] Construire le Trie à partir des keyBindings du profil
  - [x] Reconstruire le Trie quand le profil change
  - [x] Méthodes: `find(combo)`, `is_leaf(combo)`, `has_extensions(combo)`

- [x] **8.4.2** Implémenter le ChordDetector dans `detector.rs`
  - [x] Tracker `current_combo: Vec<String>` (touches pressées, triées)
  - [x] Sur key press: ajouter à combo, vérifier si feuille → trigger ou timer
  - [x] Sur timer expire: trigger le meilleur match actuel
  - [x] Sur key release: retirer de combo

- [x] **8.4.3** Fenêtre de détection configurable
  - [x] Nouveau champ `config.chordWindowMs: u32` (défaut: 30ms)
  - [x] Range: 20-100ms dans les Settings
  - [x] Timer reset à chaque nouvelle touche pressée

- [x] **8.4.4** Optimisation latence conditionnelle
  - [x] 0ms si la touche est une feuille (pas d'extensions dans le profil)
  - [x] 0ms si le combo actuel est une feuille (trigger immédiat)
  - [x] Timer seulement si des extensions sont possibles

- [x] **8.4.5** Format et normalisation des combos
  - [x] Ordre: Modifiers d'abord (Ctrl > Shift > Alt), puis base keys alphabétiques
  - [x] "KeyZ+KeyA" → normalisé en "KeyA+KeyZ"
  - [x] "Ctrl+KeyZ+KeyA" → "Ctrl+KeyA+KeyZ"

- [x] **8.4.6** UI pour capturer les multi-key chords
  - [x] KeyCaptureSlot: déjà supporte multi-key via pressedKeysRef
  - [x] Afficher preview: "A + Z" pendant la capture
  - [x] `keyCodeToDisplay("KeyA+KeyZ")` → "A+Z"

- [x] **8.4.7** Frontend `useKeyDetection.ts`
  - [x] Parser les combos multi-key reçus du backend
  - [x] Chercher le binding correspondant dans le profil

**Avantage combinatoire:**
| Type | Combinaisons (~50 touches) |
|------|----------------------------|
| 1 touche | 50 |
| 2 touches | 1,225 |
| 3 touches | 19,600 |
| + Modifiers (×8) | ×8 pour chaque |

## 8.5 Modificateur Momentum Configurable ⏳ EN DISCUSSION

Permettre à l'utilisateur de choisir quel modificateur déclenche le momentum.

> **Note**: En discussion. Cette feature résoudrait le problème Numpad+Shift (limitation hardware où Shift+Numpad4 envoie ArrowLeft au lieu de Numpad4).

- [ ] **8.5.1** Ajouter le champ config `momentumModifier`
  - [ ] Type: `"Shift" | "Alt" | "Ctrl" | "None"`
  - [ ] Défaut: "Shift" (comportement actuel)
  - [ ] "None" = momentum désactivé (utiliser Auto-Momentum toggle)

- [ ] **8.5.2** Ajouter dropdown dans Settings
  - [ ] Sous la section "Key Detection"
  - [ ] Label: "Momentum Modifier"
  - [ ] Options: Shift (default), Alt, Ctrl, None

- [ ] **8.5.3** Mettre à jour backend (`detector.rs`)
  - [ ] Lire le modifier configuré au lieu de hardcoder Shift
  - [ ] `with_momentum: bool` basé sur le modifier configuré

- [ ] **8.5.4** Mettre à jour frontend (`useKeyDetection.ts`)
  - [ ] Lire `config.momentumModifier`
  - [ ] Vérifier le modifier correspondant pour déclencher momentum

**Options:**
| Modifier | Avantage | Inconvénient |
|----------|----------|--------------|
| Shift (défaut) | Intuitif | Conflit Numpad |
| Alt | Fonctionne partout | Moins naturel |
| Ctrl | Fonctionne partout | Conflits système possibles |
| None | Simple | Perd la flexibilité |
