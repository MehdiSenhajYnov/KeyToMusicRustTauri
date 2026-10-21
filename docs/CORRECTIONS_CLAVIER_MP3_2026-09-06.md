# Correctifs après utilisation réelle : clavier et MP3

## Lettres A/Q et Z/W au démarrage

Les affectations utilisent des codes de positions physiques (`KeyA`, `KeyQ`, etc.).
L'ancien affichage utilisait leurs noms QWERTY au démarrage puis apprenait les
caractères de la disposition courante touche par touche. En AZERTY, cela produisait
des lettres inversées, puis deux A affichés après la première frappe.

La disposition complète est maintenant chargée avant le premier rendu : API
clavier du navigateur lorsqu'elle est disponible, sinon keymap native GDK sous
Linux, y compris pour WebKitGTK. Les libellés sont remplacés ensemble et les vues
sont actualisées lors d'un changement de disposition système. Le retour du focus
déclenche également une vérification. Les labels sont conservés localement comme
solution de repli quand les API ne sont pas disponibles.

Les caractères obtenus avec Shift ou AltGr ne remplacent plus le libellé de base.
Les codes d'affectation et les sons ne sont pas modifiés.

Fichiers principaux : `src-tauri/src/keys/layout.rs`, `src/utils/keyboardLayout.ts`,
`src/utils/keyMapping.ts`, `src/hooks/useKeyboardLayout.ts`, `src/main.tsx`.

## Avertissements pendant les formes d'onde MP3

Le calcul rapide sautait à une centaine de positions dans chaque fichier et
réinitialisait le décodeur à chaque saut. Les trames MP3 dépendent de données des
trames précédentes via un réservoir de bits. Cette stratégie produisait des
avertissements `invalid main_data_begin` et `skipping junk` sur un fichier valide.

Les MP3 sont désormais décodés séquentiellement pour calculer leur forme d'onde,
avec des mises à jour de progression. Le résultat continue d'être mis en cache.
Les autres codecs conservent l'analyse par échantillonnage. La détection repose sur
le codec identifié, pas seulement sur l'extension du fichier.

Le premier calcul d'un long MP3 peut prendre plus de temps ; les échantillons ne
sont pas tous conservés en mémoire. Aucun filtre ne masque les avertissements du
décodeur. Des erreurs provenant d'un fichier réellement endommagé ou d'un autre
chemin de décodage peuvent donc toujours être signalées.

Fichier principal : `src-tauri/src/audio/analysis.rs`.

## Vérifications

- Test de régression avec un MP3 synthétique valide : silence, tonalité, silence.
  Le test échouait avant la correction en capturant les avertissements ; il passe
  après correction, vérifie la forme d'onde et la progression, et ne capture aucun
  avertissement de réservoir ou de données parasites.
- 25 tests frontend réussis, dont 7 sur la disposition clavier.
- 60 tests Rust réussis. Le test nécessitant un serveur graphique est séparé de
  la suite habituelle et a également été exécuté avec succès sous Xvfb configuré
  en français, avant toute frappe.
- 14 scénarios UI réussis avec un backend simulé, dont le démarrage et rechargement
  en AZERTY, la stabilité des labels pendant la lecture et les changements de
  disposition sans modification des affectations.
- Build web et compilation native Linux réussis ; Clippy passe avec 49
  avertissements de code inutilisé/style encore présents dans le projet.
- Lancement de l'application native avec un profil temporaire sous X11 français :
  [capture avant toute frappe](audit-2026-09-06/azerty-startup.png). Les cartes
  affichent Q pour `KeyA` et A pour `KeyQ` ; le calcul réel des formes d'onde du
  MP3 de test termine sans les avertissements signalés.

Commandes spécifiques :

```bash
KTM_TEST_AZERTY=1 npm run test:ui
```

```bash
xvfb-run -a --server-args='-screen 0 1280x900x24 -noreset' \
  env -u WAYLAND_DISPLAY GDK_BACKEND=x11 bash -c \
  'setxkbmap fr && cargo test --manifest-path src-tauri/Cargo.toml french_system_layout_before_any_keypress -- --ignored --test-threads=1'
```

Le test graphique vérifie réellement la keymap Linux/X11. Le chemin Wayland
utilise la même API GDK mais n'a pas fait l'objet d'une session de recette Wayland
interactive. Les autres plateformes utilisent l'API navigateur ou le cache des
libellés ; cette correction n'ajoute pas de lecteur natif Windows/macOS.

Documentation consultée : [keymap GDK](https://docs.gtk.org/gdk3/class.Keymap.html)
et code local de `symphonia-bundle-mp3` 0.5.5 (`demuxer.rs`, `layer3/mod.rs`).
