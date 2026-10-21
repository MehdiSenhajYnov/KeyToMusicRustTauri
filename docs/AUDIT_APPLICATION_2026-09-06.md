# Audit de l’application — 6 septembre 2026

Suivi après utilisation réelle : [correctifs de l'affichage AZERTY et des avertissements MP3](CORRECTIONS_CLAVIER_MP3_2026-09-06.md).

L’audit a conduit à des corrections dans 58 fichiers de l’application et de l’extension, ainsi qu’à l’ajout de tests, d’un parcours UI reproductible et de cette documentation. Les changements portent principalement sur la conservation des données, la cohérence de la lecture audio et les actions quotidiennes : changer de profil, ajouter un son, essayer une préécoute, modifier des affectations, importer, exporter et fermer l’application.

Le dépôt contenait déjà un travail conséquent non commité, notamment sur Manga Mood et RealTest. Cet état a été conservé ; les changements décrits ici correspondent aux corrections de cet audit. Aucun commit ni publication n’a été effectué. Les tests natifs ont utilisé des répertoires de données temporaires.

## 1. Profils, sauvegardes et fermeture

- **Sauvegarde automatique centralisée.** Chaque modification du profil passe par une file de sauvegarde avec temporisation. Les écritures sont séquentielles : une sauvegarde lente ne peut plus remplacer une modification plus récente. En cas d’échec, la dernière version reste disponible en mémoire pour réessayer.
- **État visible dans l’en-tête.** L’application indique la sauvegarde en cours, sa réussite ou les changements non enregistrés, avec une action de nouvelle tentative.
- **Transitions protégées.** Changer de profil, dupliquer le profil actif, l’exporter ou fermer la fenêtre attend la sauvegarde. Un échec ne fait plus disparaître silencieusement les modifications en cours.
- **Chargements concurrents.** Si l’on sélectionne plusieurs profils rapidement, seul le dernier chargement demandé peut devenir actif. Les analyses et vérifications tardives ne modifient pas le nouveau profil.
- **Suppression d’un profil inactif.** Elle conserve la sélection du profil actif. Les sélections de touches et les brouillons propres à l’ancien profil sont réinitialisés lors d’un changement effectif.
- **Sons non affectés conservés.** Le chargement ne supprime plus les sons simplement parce qu’aucune touche ne leur est actuellement affectée.
- **Duplication indépendante.** Le profil dupliqué reçoit de nouveaux identifiants de pistes et de sons ; les affectations sont remappées. Les fichiers audio sources restent partagés.
- **Erreurs de démarrage compréhensibles.** Un échec de chargement initial présente une action Retry. Un dernier profil illisible est signalé tout en laissant accès à la liste des profils.
- **Noms et classement.** Les noms vides sont rejetés et la liste des profils est triée de façon stable.

Fichiers principaux : `src/stores/profileStore.ts`, `src/utils/profilePersistence.ts`, `src/components/Profiles/ProfileSelector.tsx`, `src/App.tsx` et `src-tauri/src/storage/profile.rs`.

## 2. Stockage, import et export

- **Écritures atomiques.** Configuration, profils et index du cache utilisent un fichier temporaire unique dans le répertoire de destination, synchronisé avant remplacement. Une erreur d’écriture ne commence plus par supprimer la destination existante.
- **Modifications backend préservées.** Les écritures de profils sont verrouillées ; les mises à jour Discovery des vidéos refusées et des identifiants résolus ne sont plus écrasées par une ancienne copie du frontend.
- **Identifiants de profils contrôlés.** Ils ne peuvent pas servir à sortir du répertoire des profils.
- **Import préparé avant publication.** L’archive est vérifiée, extraite dans un répertoire temporaire, puis publiée. Les fichiers publiés sont retirés si l’enregistrement final échoue.
- **Validation des archives.** Contrôle de la version lorsqu’elle est présente, des références entre pistes, sons et touches, des identifiants, durées, volumes et du nombre de pistes. Limites : 32 Mio par document JSON, 4 Gio par fichier audio et 16 Gio d’audio extrait au total.
- **Collisions de fichiers évitées.** Deux sons nommés pareil dans des dossiers distincts restent distincts. Une même entrée audio référencée plusieurs fois est extraite une seule fois. Les noms de destination sont générés et les identifiants remappés ; les chemins fournis par l’archive ne deviennent pas des chemins de destination libres.
- **Export en flux.** Les fichiers audio sont copiés par blocs de 64 Kio, avec vérification de l’annulation pendant la copie. Un gros fichier n’est plus intégralement chargé en mémoire pour être exporté.
- **Destination d’export préservée.** Échec ou annulation laisse intact un fichier préexistant au même emplacement. Le suivi des exports interrompus est nettoyé après une sortie normale.
- **Cycle d’export complet.** Protection contre les doubles lancements, libération des écouteurs même en cas d’erreur, état cohérent pendant le sélecteur de destination. À la fermeture, l’application attend la fin effective de l’annulation.

Fichiers principaux : `src-tauri/src/storage/`, `src-tauri/src/import_export/` et `src/stores/exportStore.ts`.

## 3. Lecture audio, pistes et préécoute

- **Volumes de pistes appliqués dès la première lecture.** Le volume enregistré est transmis au moteur, y compris si la piste audio n’existait pas encore.
- **Chargements audio annulables.** Le décodage initial est déplacé hors du traitement IPC synchrone. Stop ou une lecture plus récente invalide une préparation lente : elle ne peut plus relancer ensuite un ancien son.
- **Réponse après démarrage effectif.** Le moteur confirme que la lecture a démarré ; une commande simplement placée dans une file n’est plus considérée comme un succès. Les erreurs sont renvoyées à l’appelant.
- **Arrêt et ressources.** L’arrêt libère les pistes et les associations de sons concernées. Passer successivement par de nombreux profils ne consomme plus définitivement la limite de 20 pistes.
- **Préécoute dédiée.** Le formulaire d’ajout utilise une piste temporaire séparée des pistes musicales. Fermer le formulaire arrête cette préécoute sans arrêter une piste du profil. Elle fonctionne avant même de choisir une piste de destination et ne consomme pas le quota des pistes normales.
- **Événements tardifs filtrés.** La fin d’un ancien son ne remet plus à l’arrêt son remplaçant. Une modification tardive du volume de l’ancien son ne change plus celui du nouveau.
- **Décodage plus robuste.** Rejet des sources vides ou sans premier bloc décodable, gestion des blocs sans échantillons et correction du dimensionnement des buffers multicanaux.
- **Boucles.** Normalisation des index invalides, négatifs ou hors limites ; les annulations attendues ne produisent pas de faux messages d’erreur.
- **Suppression de piste.** Elle arrête également sa lecture. Les contrôles de plusieurs pistes ne partagent plus un minuteur susceptible de faire perdre une modification.

Fichiers principaux : `src-tauri/src/audio/engine.rs`, `src-tauri/src/audio/symphonia_source.rs`, `src-tauri/src/commands.rs`, `src/stores/audioStore.ts` et les hooks de lecture.

## 4. Ajouter, affecter et modifier les sons

- **Un ajout = une annulation.** Ajouter un lot de sons, créer sa piste et affecter ses touches constitue une seule action d’historique. Ctrl+Z annule l’ensemble.
- **Abandon explicite d’un ajout.** Fermer un formulaire contenant des fichiers ou des téléchargements demande confirmation. Les téléchargements déjà lancés peuvent terminer dans le cache.
- **Validation avant modification.** Les touches sont dédupliquées et la limite de pistes est contrôlée avant d’ajouter des éléments au profil.
- **Édition de touches multipistes.** Des onglets permettent de choisir la piste à éditer quand une même touche déclenche plusieurs pistes. La liste des sons et la forme d’onde suivent cette sélection.
- **Déplacement et fusion cohérents.** Les affectations restent uniques pour un couple touche/piste ; une fusion déduplique les sons sans écraser les affectations d’autres pistes.
- **Suppressions ciblées.** Retirer un son d’une affectation ne le supprime du profil que s’il n’est plus utilisé ailleurs. La suppression d’une sélection ne nettoie pas au passage tous les autres sons non affectés.
- **Réparation des fichiers manquants.** Le remplacement recalcule la durée et réinitialise le point de départ ; les résultats tardifs sont ignorés si le profil ou l’élément à réparer a changé. Les erreurs sont visibles et la réparation est sauvegardée avant fermeture.
- **Historique au clavier.** Ctrl/Cmd+Shift+Z fonctionne avec la touche Z majuscule et les raccourcis d’historique respectent les champs de texte et dialogues ouverts.

Fichiers principaux : `src/components/Sounds/`, `src/components/Errors/FileNotFoundModal.tsx`, `src/utils/keyBindings.ts`, `src/stores/historyStore.ts` et `src/hooks/useUndoRedo.ts`.

## 5. Clavier, dialogues, accessibilité et réglages

- **Suspension temporaire distincte du réglage utilisateur.** Écrire dans un champ, capturer un raccourci ou ouvrir un dialogue suspend la détection sans enregistrer « désactivé » dans la configuration. Plusieurs causes de suspension peuvent coexister ; en fermer une ne réactive pas les autres.
- **Suspension côté moteur et interface.** Elle couvre la détection native et le repli navigateur. Les touches et accords en attente sont vidés pour éviter des déclenchements différés.
- **Navigation des dialogues.** Focus placé dans le dialogue, Tab contenu dans celui-ci, Échap réservé au dialogue supérieur et restauration du focus à sa fermeture. La capture d’un raccourci dans Settings est annulée avant de fermer Settings.
- **Confirmations fiables.** Suppression de l’expiration automatique à 30 secondes ; une nouvelle demande termine proprement l’ancienne. Cancel reçoit le focus initial.
- **Lisibilité et accès au clavier.** Contraste du texte atténué augmenté, focus visible, prise en compte de la préférence de réduction des animations, profils accessibles au clavier, actions visibles au focus et liste de profils défilante.
- **Notifications accessibles.** Annonce des toasts, libellés des boutons de fermeture et affichage au-dessus des confirmations.
- **Réglages ordonnés.** Les commandes sont sérialisées. En cas d’échec, seuls les champs encore concernés sont restaurés ; un ancien échec ne remet pas en arrière une modification plus récente.
- **Cohérence backend après erreur.** Un échec de persistance restaure aussi les champs et l’état du moteur concernés, en préservant les autres réglages. La fermeture attend la file frontend puis force l’écriture de la configuration backend.
- **Volume général réactif.** Suppression d’une temporisation frontend inutile sur son contrôle.

Fichiers principaux : `src/hooks/useDialog.ts`, `src/hooks/useTextInputFocus.ts`, `src/utils/keyDetectionSuspension.ts`, `src/stores/settingsStore.ts`, `src-tauri/src/keys/detector.rs`, `src-tauri/src/state.rs`, `src/index.css` et `tailwind.config.js`.

## 6. YouTube, Discovery et Manga Mood

- **URLs YouTube analysées correctement.** Validation du domaine et de l’identifiant vidéo ; prise en charge des liens watch, courts, shorts, live et embed. Les faux domaines et arguments déguisés sont rejetés. Le téléchargement utilise une URL canonique après le séparateur d’arguments `--`.
- **Téléchargements concurrents maîtrisés.** Verrou par vidéo pour éviter deux écritures du même cache et limite globale de trois téléchargements. Installation de yt-dlp et de FFmpeg protégée contre les lancements simultanés ; remplacement de yt-dlp atomique.
- **Recherche et playlists.** Lecture des deux flux du processus pour éviter les blocages, délai limite de 90 secondes, vérification du code de sortie et conservation de la vérification TLS. Les résultats d’une ancienne recherche ne remplacent plus ceux de la requête courante.
- **Premier démarrage plus sobre.** L’usage de sons locaux sur une installation neuve ne lance plus automatiquement l’installation de yt-dlp. La mise à jour au démarrage reste possible si l’outil est déjà installé.
- **Cache référencé conservé.** La recherche des fichiers encore utilisés inclut les sources locales. Refuser une vidéo ne supprime pas son audio s’il est encore référencé par un profil enregistré.
- **Discovery isolé par exécution.** Une nouvelle recherche dispose de son propre état d’annulation. Une ancienne exécution ne se réactive pas au redémarrage de la suivante et ses résultats tardifs sont ignorés. Un téléchargement terminé après un changement de profil n’ajoute pas son résultat au mauvais profil.
- **Cycle de vie Mood sérialisé.** Démarrage, arrêt et changements de configuration du service ne se concurrencent plus librement. Les boutons évitent les démarrages, installations et téléchargements de modèles répétés.
- **Requêtes d’images acceptées.** La limite HTTP est portée à 32 Mio pour les fenêtres de plusieurs images, qui pouvaient dépasser la limite Axum par défaut de 2 Mio.
- **Choix audio par piste.** Parmi les affectations éligibles d’une même piste, l’intensité la plus forte est retenue. Les états de lecture et de mood engagé ne sont annoncés qu’après une lecture réussie ; un échec permet une nouvelle tentative.
- **Écouteurs nettoyés.** Les abonnements aux événements de statut sont libérés même si leur installation asynchrone termine après la fermeture de la vue.
- **Port de l’extension validé.** Entier compris entre 1024 et 65535, contrôlé dans le popup et le background ; les erreurs d’enregistrement sont renvoyées à l’interface.

Fichiers principaux : `src-tauri/src/youtube/`, `src/components/Discovery/DiscoveryPanel.tsx`, `src/hooks/useMoodPlayback.ts`, `src/stores/moodStore.ts`, `src-tauri/src/mood/server.rs` et `WebExtension/manga-mood/{background,popup}.js`.

## 7. Outillage et validation

- Ajout de Vitest et de quatre fichiers de tests frontend.
- Ajout de `scripts/smoke-ui.mjs` avec Playwright Core. Il démarre son propre Vite, injecte un backend simulé, vérifie les parcours et produit des captures dans un répertoire temporaire.
- Passage à Vite 6.4.3, Vitest 3.2.7 et mise à jour des dépendances transitives. L’audit npm final ne signale aucune vulnérabilité connue dans l’arbre installé. Avis consultés : [Vitest](https://github.com/advisories/GHSA-5xrq-8626-4rwp) et [Vite](https://github.com/advisories/GHSA-fx2h-pf6j-xcff).
- Découpage effectif du formulaire d’ajout en module chargé à la demande ; suppression des imports dynamiques inutiles des événements Tauri. Le build web final ne produit plus les avertissements correspondants.
- Cibles de bundle Tauri adaptées à la plateforme (`all`) au lieu d’imposer uniquement NSIS.
- Ajout des commandes de validation au [référentiel de commandes](COMMANDS_REFERENCE.md).

| Vérification exécutée | Résultat |
| --- | --- |
| `npm test` | 18 tests réussis dans 4 fichiers |
| `npm run build` | TypeScript et bundle de production réussis |
| `cargo test --manifest-path src-tauri/Cargo.toml` | 59 tests réussis ; 43 au début de l’audit |
| `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` | Conforme |
| `cargo clippy --manifest-path src-tauri/Cargo.toml` | Réussi avec 50 avertissements, comme au départ ; principalement code inutilisé et suggestions de style/API dans les modules Mood/RealTest |
| `npm audit` | 0 vulnérabilité signalée |
| `node --check` sur background, popup et content de l’extension | Réussi |
| Parcours Playwright avec Chromium/Helium | 11 scénarios réussis, aucune erreur JavaScript non interceptée |
| `npm run tauri -- build --debug --no-bundle` | Exécutable natif Linux construit |
| Démarrage natif sous Xvfb, données vierges isolées | Fenêtre rendue, initialisation du moteur audio journalisée, aucun téléchargement de binaire au démarrage |

Les nouveaux tests Rust couvrent les générations de lecture, la validation des URLs, le verrou par vidéo, les écritures concurrentes, les identifiants et duplications de profils, les imports avec collisions et échecs de publication, les exports réussis/annulés/échoués et la restauration ciblée de configuration.

Les tests frontend couvrent l’ordre des sauvegardes et leur reprise, les changements concurrents de profil, la suppression d’un profil inactif, la conservation des sons non affectés, les restaurations de réglages concurrentes, les affectations, les index de boucle, les fins de lecture tardives et les confirmations remplacées.

Les onze scénarios UI sont : démarrage ; lecture multipiste avec les volumes enregistrés ; suspension des raccourcis dans Settings ; Échap et confinement du focus ; préécoute dédiée ; confirmation d’abandon du formulaire ; ajout annulable en une action ; échec de sauvegarde suivi de Retry ; absence de débordement horizontal à 800 × 600 ; navigation entre pistes d’une touche ; suppression d’un profil inactif.

### Captures

- [Démarrage de l’application native avec bibliothèque vierge](audit-2026-09-06/native-startup.png).
- [Bibliothèque avec données de test](audit-2026-09-06/library.png).
- [Édition d’une touche multipiste à 800 × 600](audit-2026-09-06/multi-track-details.png).
- [Réglages](audit-2026-09-06/settings.png).

### Limites de cette validation

Le test UI utilise un backend simulé : il valide les interactions et commandes envoyées, pas le son réellement entendu. Le lancement natif confirme le rendu et le démarrage ; il ne remplace pas une session d’écoute avec plusieurs périphériques et changements de sortie.

Les installateurs Windows/macOS/Linux, les permissions et raccourcis globaux Wayland sur une session utilisateur réelle, ainsi qu’un parcours complet navigateur → pages de manga → modèle sur GPU → audio, restent à vérifier dans leurs environnements respectifs. Le benchmark RealTest complet et ses téléchargements de modèles n’ont pas été lancés. La mémoire consommée par de très longs chapitres reste également à mesurer en situation réelle. Les avertissements Rust existants sont conservés et explicitement signalés ci-dessus.

Pour une recette matérielle, ouvrir un profil existant, jouer deux pistes, faire varier leurs volumes, changer de sortie audio pendant la lecture, tester Stop pendant le chargement d’un gros fichier, puis exporter/réimporter le profil. Pour Manga Mood, vérifier un chapitre complet et le changement de chapitre avec l’extension et le modèle effectivement installés. Fermer puis rouvrir l’application doit restaurer les derniers réglages et modifications enregistrés.
