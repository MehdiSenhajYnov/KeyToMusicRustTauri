# KeyToMusic — Spécification fonctionnelle

> État fonctionnel de l’application au 27 juillet 2026.
>
> Ce document décrit ce que l’application permet de faire du point de vue de
> l’utilisateur. Il ne détaille ni le code, ni l’architecture, ni les
> technologies employées.
>
> Conformément au périmètre demandé, Manga Mood et la détection automatique
> d’ambiance par IA ne sont pas couverts.

## 1. Résumé de l’application

KeyToMusic est une soundboard destinée en premier lieu à accompagner la lecture
de mangas avec de la musique, des ambiances et des effets sonores.

L’utilisateur prépare des profils dans lesquels il :

- organise ses sons en pistes, par exemple `OST`, `Ambiance` et `SFX` ;
- associe un ou plusieurs sons à des touches ou à des combinaisons de touches ;
- choisit la manière dont plusieurs sons associés à une même touche doivent
  s’enchaîner ;
- règle les volumes, les transitions et les positions de départ ;
- déclenche ensuite ses sons au clavier, y compris lorsque KeyToMusic n’est pas
  la fenêtre active.

L’objectif est de pouvoir changer rapidement de musique pendant une lecture
sans quitter le navigateur, tout en conservant des transitions sonores propres.
L’application peut aussi servir de soundboard générale pour du jeu de rôle, du
streaming, une présentation ou toute activité nécessitant des sons déclenchés
au clavier.

L’application vise Windows 10/11, macOS 10.15 ou plus récent et les principales
distributions Linux.

## 2. Modèle fonctionnel

### 2.1 Profil

Un profil est une configuration complète et indépendante. Il contient :

- son nom ;
- ses pistes ;
- ses sons ;
- ses assignations de touches ;
- les volumes et positions de départ propres aux sons ;
- les modes de lecture des touches ;
- son historique de recommandations refusées.

Un profil peut représenter un manga, une série, un arc narratif, une session ou
un thème sonore particulier.

### 2.2 Piste

Une piste est un canal de lecture logique. Une piste ne joue qu’un son principal
à la fois, mais plusieurs pistes différentes peuvent jouer simultanément.

Exemple :

- la piste `OST` joue une musique ;
- la piste `Ambiance` joue de la pluie ;
- la piste `SFX` joue un effet ponctuel.

Déclencher un nouveau son sur une piste remplace progressivement le son qui y
jouait déjà. Déclencher un son sur une autre piste permet de le superposer aux
sons déjà en lecture.

### 2.3 Son

Un son possède :

- un nom ;
- une source locale ou YouTube ;
- une durée ;
- un volume individuel ;
- une position de départ appelée `Momentum`.

Un même son peut faire partie d’une assignation contenant plusieurs sons.

### 2.4 Assignation de touche

Une assignation relie :

- une touche, une combinaison avec modificateurs ou un accord de plusieurs
  touches ;
- une piste ;
- un ou plusieurs sons ;
- un mode de lecture ;
- éventuellement un nom de groupe personnalisé.

Une même touche peut avoir une assignation sur plusieurs pistes. Dans ce cas,
un seul appui déclenche toutes les pistes concernées en parallèle.

### 2.5 Momentum

Le Momentum est une position enregistrée à l’intérieur d’un son. Il sert à
éviter une introduction trop longue et à démarrer directement au passage le
plus utile, par exemple le début d’un refrain, d’une montée ou d’un combat.

Le son peut toujours être lancé depuis le début ou depuis son Momentum selon
les réglages et la manière dont la touche est pressée.

## 3. Parcours utilisateur principal

Le parcours standard est le suivant :

1. Créer un profil.
2. Créer une ou plusieurs pistes.
3. Ajouter des fichiers audio locaux ou des sons venant de YouTube.
4. Assigner ces sons à une ou plusieurs touches.
5. Régler le volume, le mode de lecture et le Momentum de chaque son.
6. Laisser KeyToMusic ouvert et retourner à sa lecture.
7. Déclencher les musiques et ambiances au clavier sans revenir dans
   l’application.

## 4. Organisation générale de l’interface

### 4.1 Barre supérieure

La barre supérieure donne accès à :

- l’identité de l’application ;
- le volume général, de 0 à 100 % ;
- l’aide sur les raccourcis clavier ;
- les réglages.

Le volume général peut être ajusté au curseur ou avec la molette de la souris
lorsque le pointeur est au-dessus du curseur.

### 4.2 Barre latérale

La barre latérale regroupe :

- la liste des profils ;
- les interrupteurs `Auto-Momentum` et `Key Detection` ;
- le bouton `Stop All` ;
- la liste des sons actuellement joués ;
- le panneau de découverte musicale.

### 4.3 Zone principale

La zone principale affiche :

- les pistes et leur volume ;
- la recherche dans les assignations ;
- la grille des touches assignées ;
- le bouton d’ajout de sons ;
- le panneau de détail de la ou des touches sélectionnées.

Le panneau de détail peut être redimensionné verticalement en faisant glisser
sa barre de séparation.

### 4.4 États vides guidés

L’application accompagne les premiers usages :

- sans profil, elle propose directement d’en créer un ;
- sans piste, elle propose de créer une première piste avec un exemple de nom ;
- sans assignation, elle propose d’ajouter un son ou de déposer des fichiers
  audio.

## 5. Gestion des profils

### 5.1 Création et sélection

- Création d’un profil depuis la barre latérale ou l’écran vide initial.
- Saisie du nom directement dans l’interface.
- Validation avec `Entrée` et annulation avec `Échap`.
- Sélection instantanée d’un profil dans la liste.
- Mémorisation du dernier profil actif.

Changer de profil :

- arrête tous les sons en cours ;
- charge les pistes, sons et touches du nouveau profil ;
- remet à zéro la sélection de touches, la recherche et l’historique
  d’annulation ;
- vérifie la présence des fichiers audio ;
- prépare en arrière-plan les durées et formes d’onde manquantes.

### 5.2 Renommage

Un double-clic sur le nom d’un profil permet de le renommer. La modification
peut être validée avec `Entrée`, en quittant le champ, ou annulée avec `Échap`.

### 5.3 Duplication

Un profil peut être dupliqué depuis son action dédiée. La copie :

- reçoit un nouveau nom avec le suffixe `(Copy)` ;
- conserve les pistes, sons, assignations et réglages du profil ;
- devient un profil indépendant pour les modifications futures ;
- ne reprend pas la liste des recommandations musicales refusées.

Les sources audio référencées ne sont pas dupliquées physiquement lors de cette
opération.

### 5.4 Suppression

La suppression demande une confirmation. Elle retire le profil et son historique
de découverte. Les fichiers YouTube mis en cache et devenus inutilisés peuvent
ensuite être nettoyés automatiquement.

La suppression d’un profil n’efface jamais les fichiers audio personnels
d’origine.

## 6. Gestion des pistes

### 6.1 Création

- Création depuis l’état vide ou avec `+ Add Track`.
- Nom libre, avec des exemples comme `OST`, `Ambiance` ou `SFX`.
- Validation avec `Entrée`, annulation avec `Échap`.
- Maximum de 20 pistes dans un profil.

### 6.2 Renommage

Un double-clic sur le nom d’une piste active sa modification. La saisie est
validée avec `Entrée` ou en quittant le champ.

### 6.3 Volume par piste

Chaque piste possède un volume de 0 à 100 %. Le curseur accepte aussi la
molette de la souris. La modification s’applique à la lecture en cours.

### 6.4 Suppression

La suppression d’une piste demande confirmation et entraîne :

- la suppression de toutes les assignations appartenant à cette piste ;
- la suppression, dans le profil, des sons qui ne sont plus utilisés par
  aucune autre assignation.

Les fichiers locaux d’origine ne sont pas supprimés du disque.

### 6.5 Superposition et remplacement

- Plusieurs pistes peuvent jouer en même temps.
- Une piste donnée ne joue qu’un son principal à la fois.
- Un nouveau son lancé sur la même piste remplace le précédent avec un fondu.
- Une même touche assignée à plusieurs pistes les déclenche simultanément.

## 7. Ajout de sons locaux

### 7.1 Sélecteur de fichiers

Le bouton `Add Files` permet de choisir un ou plusieurs fichiers en une seule
opération.

Les formats proposés par l’application sont :

- MP3 ;
- WAV ;
- OGG ;
- FLAC ;
- M4A ;
- AAC ;
- WebM.

### 7.2 Glisser-déposer

Un ou plusieurs fichiers audio peuvent être déposés directement dans la fenêtre.
Une zone de dépôt visuelle apparaît pendant le glisser-déposer. Les fichiers
non audio sont ignorés.

### 7.3 Import multiple

Lors de l’ajout de plusieurs fichiers :

- chaque fichier obtient son propre Momentum ;
- les durées sont calculées en parallèle ;
- les formes d’onde apparaissent progressivement ;
- plusieurs touches peuvent être capturées ;
- les fichiers sont distribués à tour de rôle entre les touches fournies.

Exemple : avec cinq sons et deux touches `A` et `Z`, les sons sont associés dans
l’ordre à `A`, `Z`, `A`, `Z`, `A`.

Si plusieurs sons arrivent sur la même touche et la même piste, ils sont
regroupés dans la même assignation.

### 7.4 Référence aux fichiers

Dans un profil normal, KeyToMusic référence les fichiers locaux à leur
emplacement actuel. Il ne les copie pas automatiquement. Déplacer ou supprimer
un fichier après son ajout peut donc rendre le son introuvable.

## 8. Ajout de sons depuis YouTube

### 8.1 Installation assistée

Au premier usage, l’application vérifie la présence de l’outil nécessaire aux
téléchargements et propose de l’installer depuis l’interface. Le composant
nécessaire à la conversion audio est également installé automatiquement au
besoin.

L’outil de téléchargement est vérifié et actualisé périodiquement afin de
limiter les erreurs liées aux changements de YouTube.

### 8.2 Ajout par URL

L’utilisateur peut coller :

- l’URL d’une vidéo YouTube ;
- une URL courte `youtu.be` ;
- une URL YouTube Shorts ;
- une URL de playlist.

L’audio est téléchargé, son titre est récupéré, sa durée est calculée, puis le
son rejoint la liste des fichiers à configurer avant ajout au profil.

### 8.3 Recherche intégrée

Le champ YouTube accepte aussi une recherche textuelle :

- recherche automatique après une courte pause de saisie ;
- recherche manuelle avec le bouton ou `Entrée` ;
- affichage de six résultats ;
- titre, chaîne et durée pour chaque résultat ;
- indication lorsqu’un résultat est déjà téléchargé ;
- ajout direct d’un résultat.

### 8.4 Préécoute d’un résultat

Un résultat de recherche peut être écouté avant téléchargement :

- démarrage et pause ;
- barre de progression ;
- déplacement dans la vidéo ;
- affichage du temps actuel et de la durée ;
- fermeture de la préécoute ;
- nouvelle tentative en cas d’erreur de lecture.

Les premières préécoutes sont préparées en arrière-plan pour réduire l’attente.

### 8.5 Import de playlist

Pour une playlist :

- l’application affiche son titre et le nombre de vidéos disponibles ;
- les vidéos privées, supprimées ou indisponibles sont écartées ;
- toutes les entrées sont sélectionnées par défaut ;
- les actions `All` et `None` permettent de tout sélectionner ou désélectionner ;
- chaque vidéo peut être cochée individuellement ;
- seules les vidéos choisies sont ajoutées ;
- plusieurs téléchargements peuvent progresser simultanément.

Lorsqu’une URL désigne à la fois une vidéo et une playlist, une case permet de
choisir entre la vidéo seule et la playlist complète. Ce choix est mémorisé pour
les utilisations suivantes.

### 8.6 Progression, reprise et erreurs

Chaque téléchargement affiche :

- son état courant ;
- une animation d’activité ;
- son pourcentage lorsqu’il est disponible ;
- une barre de progression.

Les erreurs réseau temporaires peuvent être retentées automatiquement. Les
messages distinguent notamment les vidéos privées, indisponibles, bloquées dans
une région, les URL invalides et les problèmes réseau.

### 8.7 Cache YouTube

Les sons téléchargés sont mis en cache :

- une même vidéo déjà disponible n’est pas téléchargée à nouveau ;
- les entrées dont le fichier a disparu sont détectées ;
- les fichiers qui ne sont plus utilisés par un profil ou la découverte sont
  nettoyés automatiquement ;
- les téléchargements partiels sont nettoyés avant une nouvelle tentative.

## 9. Configuration avant ajout au profil

Une fois les fichiers sélectionnés ou téléchargés, la fenêtre d’ajout permet de
configurer :

- la ou les touches ;
- la piste cible ;
- éventuellement une nouvelle piste ;
- le volume initial commun ;
- le mode de lecture ;
- le Momentum individuel de chaque son.

Chaque son peut être retiré de la liste avant validation.

### 9.1 Préécoute locale

Chaque fichier peut être lu depuis son Momentum avant ajout. Modifier le
Momentum pendant cette préécoute relance rapidement le son à la nouvelle
position.

### 9.2 Création d’une piste pendant l’ajout

Si aucune piste ne convient, une nouvelle piste peut être créée directement
depuis la fenêtre sans interrompre le parcours d’ajout.

### 9.3 Ajout à une assignation existante

Si la touche et la piste sélectionnées possèdent déjà une assignation, les
nouveaux sons sont ajoutés à son groupe au lieu de remplacer les sons existants.

## 10. Formes d’onde et Momentum

### 10.1 Visualisation

Une forme d’onde représente l’énergie du morceau et facilite le choix d’une
position de départ. Elle est disponible :

- pendant l’ajout de fichiers ;
- dans le détail d’une touche ;
- dans les recommandations de la découverte.

La forme d’onde peut montrer :

- la position du Momentum ;
- la position de lecture actuelle ;
- une position de Momentum suggérée ;
- un état de chargement progressif.

### 10.2 Modification du Momentum

Le Momentum peut être réglé :

- en cliquant ou en faisant glisser le marqueur sur la forme d’onde ;
- avec un curseur classique ;
- avec la molette au-dessus du curseur ;
- avec une valeur numérique en secondes ;
- en acceptant la suggestion automatique.

Les valeurs décimales sont acceptées.

### 10.3 Suggestion de Momentum

L’application analyse l’évolution de l’énergie du morceau pour proposer une
position située près d’une montée musicale intéressante. La suggestion :

- est visuellement distincte du choix utilisateur ;
- peut être appliquée en un clic ;
- peut être appliquée automatiquement aux nouveaux sons lorsque
  `Auto-Momentum` est actif.

Cette analyse porte uniquement sur le son. Elle ne correspond pas à la
détection d’ambiance Manga Mood exclue de ce document.

### 10.4 Déclenchement du Momentum

Trois comportements sont possibles :

- `Auto-Momentum` actif : les sons démarrent automatiquement à leur Momentum ;
- `Auto-Momentum` inactif : les sons démarrent normalement à 0:00 ;
- maintien du modificateur Momentum pendant l’appui : le son démarre
  ponctuellement à son Momentum.

Le modificateur peut être `Shift`, `Ctrl`, `Alt` ou désactivé.

Une combinaison assignée exactement, par exemple `Ctrl+A`, reste prioritaire
sur l’interprétation « touche A avec modificateur Momentum ».

## 11. Modes de lecture d’une touche

Une assignation peut contenir un nombre quelconque de sons.

| Mode | Choix au déclenchement | À la fin du son |
|---|---|---|
| `Off` | Un son est choisi aléatoirement en évitant si possible la répétition immédiate | La lecture s’arrête |
| `Random` | Un son est choisi aléatoirement en évitant si possible la répétition immédiate | Un autre son aléatoire est relancé automatiquement |
| `Single` | Le même son est conservé | Le même son boucle automatiquement |
| `Sequential` | Les sons sont pris dans leur ordre | Le son suivant est lancé automatiquement, puis la séquence recommence |

Dans les modes en boucle :

- `Stop` ou `Stop All` interrompt la boucle ;
- `Random` et `Sequential` tiennent compte d’`Auto-Momentum` pour les relances ;
- en mode `Single`, une lecture démarrée au Momentum continue de boucler depuis
  ce Momentum.

Changer de mode remet l’index de sélection au début.

## 12. Détection et assignation des touches

### 12.1 Détection globale

Les touches assignées peuvent déclencher des sons même lorsque KeyToMusic est
en arrière-plan. L’utilisateur peut désactiver temporairement cette détection
avec l’interrupteur `Key Detection`.

Les raccourcis de sécurité restent disponibles même lorsque la détection des
sons est désactivée :

- arrêter tous les sons ;
- réactiver ou désactiver la détection ;
- activer ou désactiver `Auto-Momentum`.

### 12.2 Protection pendant la saisie

Quand l’utilisateur saisit du texte dans KeyToMusic, la détection sonore est
temporairement suspendue afin que les lettres tapées ne déclenchent pas de
musique. Elle est restaurée en quittant le champ.

Les curseurs et cases à cocher ne sont pas considérés comme des champs de texte.

### 12.3 Répétition et cooldown

- Le maintien d’une touche ne produit pas une succession incontrôlée de
  déclenchements.
- Un cooldown global peut ignorer les appuis trop rapprochés.
- Le cooldown est configurable de 0 à 2 000 ms dans l’interface, par pas de
  50 ms.
- Le cooldown ne démarre que lorsqu’au moins un son a réellement été lancé.

### 12.4 Touches prises en charge

L’assignation couvre notamment :

- lettres `A` à `Z` ;
- chiffres de la rangée supérieure ;
- chiffres et opérations du pavé numérique ;
- touches `F1` à `F12` ;
- flèches ;
- espace, entrée, tabulation et échappement ;
- retour arrière, suppression, insertion ;
- début, fin, page précédente et page suivante ;
- verrouillage majuscules, numérique et défilement ;
- impression écran et pause ;
- principales touches de ponctuation ;
- modificateurs gauche et droit.

### 12.5 Dispositions de clavier

L’application travaille avec la position physique des touches et apprend aussi
leur représentation dans la disposition active. Elle peut ainsi afficher
correctement les claviers AZERTY, QWERTY, QWERTZ et conserver la compatibilité
avec d’anciennes assignations.

### 12.6 Combinaisons avec modificateurs

Une assignation peut utiliser `Ctrl`, `Shift` et `Alt`, seuls ou combinés avec
une touche principale :

- `Ctrl+A` ;
- `Shift+F1` ;
- `Ctrl+Shift+Z` ;
- `Alt+Num1`.

### 12.7 Accords de plusieurs touches

Plusieurs touches principales peuvent former un véritable accord :

- `A+Z` ;
- `A+E+Z` ;
- `Ctrl+A+Z`.

Les modificateurs sont normalisés en premier et les autres touches sont
ordonnées de manière stable. Si une combinaison courte est le début possible
d’une combinaison plus longue, l’application attend brièvement pour déterminer
laquelle a été jouée.

La fenêtre de détection des accords est réglable de 20 à 100 ms :

- une valeur basse favorise la réactivité ;
- une valeur haute facilite les accords moins parfaitement simultanés.

### 12.8 Prévention des conflits

Lors de la capture d’une touche, l’application :

- bloque les raccourcis réservés à l’annulation et au rétablissement ;
- bloque les raccourcis système courants comme copier, coller, fermer ou
  quitter ;
- bloque les raccourcis globaux déjà configurés dans KeyToMusic ;
- avertit pour certaines combinaisons susceptibles de gêner le navigateur ou
  les menus du système ;
- signale les conflits entre le modificateur Momentum et les raccourcis
  globaux.

Les touches concernées affichent un avertissement persistant dans la grille.

### 12.9 Cas particulier du pavé numérique avec Shift

Avec `Verr Num` actif, certains systèmes transforment `Shift+Num4` en flèche
gauche ou en touche `Fin`. C’est un comportement matériel et système classique.

Les contournements prévus sont :

- choisir `Ctrl` ou `Alt` comme modificateur Momentum ;
- assigner explicitement la touche alternative reçue ;
- désactiver `Verr Num`.

### 12.10 Détection en arrière-plan sous Linux

L’application vérifie si la session Linux autorise la détection en arrière-plan.
Sous Wayland, elle peut :

- expliquer qu’une autorisation système est nécessaire ;
- proposer une activation automatique ;
- afficher l’invite d’administration habituelle ;
- revérifier l’accès ;
- indiquer si une déconnexion/reconnexion de session est recommandée.

## 13. Lecture audio et transitions

### 13.1 Volumes

Le niveau audible résulte de trois réglages combinés :

- volume général ;
- volume de la piste ;
- volume individuel du son.

Les modifications sont lissées afin de limiter les clics sonores et
s’appliquent à la lecture en cours.

### 13.2 Crossfade

Quand un nouveau son remplace celui qui joue sur la même piste :

- l’ancien son diminue progressivement ;
- le nouveau monte progressivement ;
- la transition cherche à conserver une puissance sonore régulière.

La durée du fondu est réglable de 100 à 2 000 ms, par pas de 50 ms.

Le crossfade est propre à chaque piste. Les pistes différentes continuent à se
superposer normalement.

### 13.3 Lecture en cours

La zone `Now Playing` affiche, pour chaque piste active :

- le nom de la piste ;
- le nom du son ;
- la position actuelle ;
- la durée totale ;
- une barre de navigation ;
- un bouton d’arrêt individuel.

Déplacer la barre relance le son à la position choisie.

L’application propose l’arrêt, mais pas de véritable pause/reprise pour les
sons joués sur les pistes principales. La préécoute YouTube possède, elle, une
commande pause/reprise.

### 13.4 Arrêt global

`Stop All` arrête immédiatement :

- toutes les pistes ;
- tous les modes de boucle ;
- toutes les lectures en cours.

Il est accessible par bouton et par raccourci global.

### 13.5 Sortie audio

L’utilisateur peut :

- sélectionner une sortie audio précise ;
- choisir `System Default` pour suivre automatiquement la sortie par défaut du
  système.

Lorsqu’une sortie change, l’application tente de reprendre chaque piste au même
endroit sur la nouvelle sortie, sans transformer ce changement en fin de son.

## 14. Grille et édition des assignations

### 14.1 Informations affichées

Chaque carte de la grille présente :

- la touche ou combinaison ;
- le nom personnalisé du groupe ou le nom du premier son ;
- le nombre de sons ;
- le nombre de pistes utilisant cette touche lorsqu’il y en a plusieurs ;
- un indicateur lorsque l’un des sons est en lecture ;
- un avertissement en cas de conflit de raccourci.

Un bref effet visuel confirme aussi la touche détectée.

### 14.2 Sélection simple

Cliquer sur une touche ouvre son panneau de détail. Cliquer une seconde fois sur
la seule touche sélectionnée ferme la sélection. Le clic sert donc à éditer la
touche ; le déclenchement du son se fait avec le clavier.

Le panneau permet de :

- renommer le groupe ;
- changer la touche complète ;
- fusionner avec une touche cible déjà occupée après confirmation ;
- changer la piste ;
- changer le mode de lecture ;
- ajouter de nouveaux sons à la touche ;
- supprimer l’assignation entière.

### 14.3 Édition de chaque son

Pour chaque son d’une touche, l’utilisateur peut :

- voir son nom et sa durée ;
- régler son volume ;
- voir sa forme d’onde ;
- régler ou appliquer son Momentum ;
- le préécouter depuis le Momentum ;
- le déplacer vers une autre touche ;
- le fusionner dans une assignation existante ;
- le retirer.

Retirer le dernier son d’une assignation retire également l’assignation vide.

### 14.4 Sélection multiple

La grille prend en charge :

- `Ctrl/Cmd + clic` pour ajouter ou retirer une touche de la sélection ;
- `Shift + clic` pour sélectionner une plage ;
- `Ctrl/Cmd + A` pour sélectionner toutes les touches actuellement visibles.

Sur plusieurs touches, il est possible de :

- changer leur piste en une seule opération ;
- changer leur mode de lecture ;
- supprimer toutes les assignations sélectionnées avec confirmation.

Si les valeurs diffèrent, le panneau affiche `Mixed`.

### 14.5 Nettoyage des sons orphelins

Lorsqu’une touche, une piste ou un son est retiré, l’application conserve les
sons encore utilisés ailleurs et retire du profil ceux qui ne sont plus
référencés.

## 15. Recherche et filtrage des assignations

La recherche s’ouvre avec le bouton dédié ou `Ctrl/Cmd + F`.

### 15.1 Recherche textuelle

Le texte saisi est recherché dans :

- le libellé de la touche ;
- le nom personnalisé du groupe ;
- les noms des sons.

Le compteur indique le nombre de correspondances sur le total. Les cartes non
correspondantes restent visibles mais atténuées et non sélectionnables.

### 15.2 Filtres avancés

Des préfixes transforment un terme en filtre :

- `t:nom` filtre sur le nom d’une piste ;
- `l:off`, `l:single`, `l:random` ou `l:sequential` filtre sur le mode ;
- `s:playing` ou `s:stopped` filtre sur l’état de lecture.

Chaque filtre devient une pastille supprimable.

### 15.3 Contrôles

- `Échap` efface toute la recherche.
- Retour arrière sur un champ vide retire la dernière pastille.
- La croix efface tous les critères.
- `Ctrl/Cmd + A` sélectionne uniquement les touches correspondant au filtre.

## 16. Annulation et rétablissement

L’application conserve jusqu’à 50 opérations de modification du profil.

Les opérations couvertes incluent notamment :

- ajout, modification et retrait de sons ;
- ajout, renommage et retrait de pistes ;
- ajout, modification, déplacement et retrait d’assignations ;
- modifications groupées de plusieurs touches.

Les raccourcis sont :

- `Ctrl+Z` sous Windows/Linux ou `Cmd+Z` sous macOS pour annuler ;
- `Ctrl+Y` sous Windows/Linux ou `Cmd+Shift+Z` sous macOS pour rétablir.

Une notification rappelle le nom de l’action annulée ou rétablie. L’état obtenu
est sauvegardé.

Ne font pas partie de cet historique :

- la création ou suppression d’un profil ;
- les réglages généraux ;
- la progression interne des modes aléatoire et séquentiel ;
- les informations techniques calculées en arrière-plan.

Changer de profil efface l’historique d’annulation courant.

## 17. Découverte musicale

### 17.1 Objectif

Le panneau `Discover` recommande de nouvelles vidéos musicales liées aux sons
déjà présents dans le profil.

Il peut utiliser :

- les vidéos YouTube du profil ;
- les fichiers locaux qu’il parvient à identifier grâce à leurs métadonnées ou
  à leur nom.

Les recommandations déjà présentes dans le profil, déjà parcourues, refusées ou
ajoutées à la liste noire sont écartées.

Une recherche exploite jusqu’à 15 sons de référence et conserve jusqu’à
30 recommandations. Les propositions de moins de 30 secondes ou de plus de
15 minutes sont écartées.

### 17.2 Lancement

- Sans son, le panneau invite à en ajouter.
- Avec des sons et aucun résultat mémorisé, la découverte démarre
  automatiquement.
- Le bouton d’actualisation présente les suggestions encore non vues.
- Si le stock est épuisé, une nouvelle recherche est lancée.
- Une recherche en cours peut être annulée.

Pour les fichiers locaux, une phase d’identification peut être affichée avant la
recherche.

### 17.3 Progression et résultats progressifs

Le panneau affiche :

- le son de référence en cours d’analyse ;
- le nombre de références traitées ;
- une barre de progression ;
- les premiers résultats avant la fin complète de la recherche.

Les résultats trouvés à partir de plusieurs sons de référence sont favorisés.
Les éléments que l’utilisateur a déjà atteints restent stables pendant l’arrivée
des nouveaux résultats.

### 17.4 Carrousel et continuité

- Les suggestions sont présentées par groupes initiaux de dix.
- Les groupes suivants sont révélés par tranches de dix.
- Les flèches de l’interface ou du clavier permettent d’avancer et reculer.
- La position, les éléments vus et le stock restant sont mémorisés par profil.
- Le stock se régénère silencieusement lorsqu’il devient faible.

Revenir sur le profil restaure donc la progression précédente au lieu de
recommencer au premier résultat.

### 17.5 Préchargement

Pour rendre la navigation fluide, l’application prépare les suggestions proches
de la position actuelle :

- jusqu’à deux derrière ;
- la suggestion actuelle ;
- jusqu’à trois devant ;
- au maximum trois téléchargements simultanés.

Les deux premières suggestions susceptibles d’apparaître après une actualisation
sont également préparées lorsque possible.

Le préchargement récupère le son, sa durée, sa forme d’onde et une suggestion de
Momentum sans encore l’ajouter au profil.

### 17.6 Carte de suggestion

Une suggestion affiche :

- son titre ;
- sa chaîne dans l’infobulle détaillée ;
- sa durée ;
- sa forme d’onde lorsqu’elle est prête ;
- l’avancement de son téléchargement ;
- son Momentum proposé ;
- une touche proposée ;
- une piste proposée.

L’utilisateur peut :

- préécouter ou arrêter le son ;
- se déplacer dans la forme d’onde ;
- modifier ou accepter le Momentum ;
- changer la touche, y compris avec une combinaison ;
- faire défiler les pistes proposées ;
- ajouter le son au profil ;
- passer à la suggestion précédente ou suivante.

Le volume de préécoute est indépendant du volume général de la future
assignation et est mémorisé.

### 17.7 Assignation intelligente

La proposition de touche et de piste s’adapte à l’organisation du profil :

- dans un profil utilisant surtout un son par touche, elle cherche la prochaine
  touche libre et la piste la moins utilisée ;
- dans un profil regroupant beaucoup de sons par touche, elle cherche le groupe
  le plus proche des sons ayant conduit à la recommandation ;
- à défaut de correspondance, elle privilégie le groupe contenant déjà le plus
  de sons.

Cette assignation reste toujours modifiable avant l’ajout.

### 17.8 Ajout d’une recommandation

Si le son a déjà été préchargé, l’ajout est immédiat. Sinon, le téléchargement
est terminé au moment de l’ajout.

Le son :

- rejoint la touche et la piste choisies ;
- est ajouté au groupe existant si cette combinaison touche/piste existe déjà ;
- conserve le Momentum préparé ;
- est retiré de la liste des suggestions après succès.

### 17.9 Refus permanent

Le bouton de refus place la vidéo dans une liste noire propre au profil. Elle ne
sera plus proposée lors des découvertes suivantes.

Les réglages permettent :

- d’afficher le nombre de vidéos refusées ;
- de voir leur titre, chaîne et durée ;
- de retirer individuellement une vidéo de la liste noire.

## 18. Sauvegarde, export et import

### 18.1 Sauvegarde courante

Les profils et réglages sont conservés localement. Les actions importantes sont
sauvegardées après modification, notamment :

- création et renommage ;
- ajout ou retrait de sons ;
- modification de piste ou de mode ;
- changements de Momentum et de volume ;
- annulation et rétablissement.

Les réglages généraux sont communs aux profils, tandis que les pistes, sons et
touches appartiennent au profil actif.

### 18.2 Export portable

`Export Profile` crée un fichier `.ktm` contenant :

- la structure complète du profil ;
- tous les fichiers audio réellement utilisés ;
- les formes d’onde déjà calculées lorsqu’elles sont disponibles ;
- les informations nécessaires à une réimportation autonome.

Les fichiers identiques utilisés plusieurs fois ne sont inclus qu’une fois. Les
conflits de noms de fichiers sont résolus automatiquement.

L’export affiche :

- le nombre de fichiers traités ;
- le fichier courant ;
- un pourcentage global ;
- un bouton d’annulation.

L’archive finale n’est remplacée qu’une fois l’export terminé. Une annulation ou
un export interrompu nettoie le fichier temporaire.

Tous les fichiers référencés doivent encore être présents au moment de
l’export. Si l’un d’eux est introuvable ou illisible, l’opération s’arrête avec
une erreur au lieu de produire une archive incomplète.

Si l’utilisateur ferme l’application pendant un export, une confirmation
prévient que le fichier serait incomplet et propose d’annuler proprement
l’opération.

### 18.3 Import d’un fichier `.ktm`

L’import :

- crée un nouveau profil au lieu d’écraser un profil existant ;
- ajoute le suffixe `(Imported)` à son nom ;
- génère des identifiants indépendants ;
- extrait les sons dans l’espace de données de l’application ;
- restaure pistes, touches, groupes, volumes, modes et Momentum ;
- réutilise les formes d’onde incluses ;
- accepte aussi les anciens exports ne contenant pas de formes d’onde.

Après import, les sons embarqués sont traités comme des fichiers locaux
autonomes. Ils ne dépendent plus du cache YouTube d’origine.

### 18.4 Import d’une sauvegarde historique

`Import Legacy Save` accepte l’ancien format JSON de KeyToMusic.

La conversion :

- crée un nouveau profil avec le suffixe `(Legacy)` ;
- crée une piste `OST` ;
- reprend les noms, chemins locaux et Momentum ;
- regroupe les anciens sons par touche ;
- convertit lettres, chiffres, touches de fonction, espace, entrée et
  ponctuation courante ;
- ignore les codes de touches anciens inconnus ;
- place les assignations importées en mode `Off`.

Les fichiers restent référencés à leur ancien emplacement et peuvent devoir
être relocalisés s’ils ont été déplacés.

## 19. Fichiers manquants et gestion des erreurs

### 19.1 Vérification des fichiers

À l’ouverture d’un profil, l’application vérifie les fichiers utilisés. Une
nouvelle erreur de lecture déclenche aussi cette vérification pour le son
concerné.

Les fichiers manquants sont regroupés dans une file afin d’être traités un par
un.

### 19.2 Fichier local manquant

Pour un fichier local, l’utilisateur peut :

- choisir son nouvel emplacement ;
- retirer le son du profil ;
- ignorer temporairement le problème ;
- ignorer tous les fichiers manquants restants.

Relocaliser le fichier met à jour la source du son sans recréer toute
l’assignation.

### 19.3 Son YouTube manquant

Pour un son YouTube dont le cache a disparu, l’utilisateur peut :

- télécharger à nouveau la vidéo d’origine ;
- retirer le son ;
- passer au problème suivant ;
- ignorer toute la file.

### 19.4 Retour utilisateur

L’application utilise :

- des notifications de succès, d’information, d’avertissement et d’erreur ;
- des confirmations avant les suppressions importantes ;
- un son d’erreur discret lorsqu’un fichier demandé est introuvable ;
- des messages simplifiés pour les erreurs audio, réseau et YouTube.

Les réglages permettent aussi d’ouvrir directement les dossiers de données et
de journaux pour le diagnostic.

## 20. Réglages disponibles

### 20.1 Valeurs initiales

Lors d’une première utilisation, les principaux réglages sont :

| Réglage | Valeur initiale |
|---|---|
| Volume général | 80 % |
| Auto-Momentum | Désactivé |
| Détection des touches | Activée |
| Stop All | `Ctrl+Shift+S` |
| Cooldown | 200 ms |
| Fenêtre des accords | 30 ms |
| Modificateur Momentum | `Shift` |
| Crossfade | 500 ms |
| Sortie audio | Sortie par défaut du système |
| Import d’une playlist entière | Désactivé |
| Volume de préécoute de la découverte | 50 % |

### 20.2 Raccourcis globaux

- `Stop All`, obligatoire et composé d’au moins deux touches.
- Activation/désactivation d’`Auto-Momentum`, optionnelle.
- Activation/désactivation de `Key Detection`, optionnelle.

La capture demande de maintenir au moins deux touches puis de les relâcher.
Les deux raccourcis optionnels peuvent être effacés.

### 20.3 Détection clavier

- cooldown global : 0 à 2 000 ms ;
- fenêtre des accords : 20 à 100 ms ;
- modificateur Momentum : `Shift`, `Ctrl`, `Alt` ou désactivé ;
- état et réparation de la détection Linux en arrière-plan.

### 20.4 Audio

- durée du crossfade : 100 à 2 000 ms ;
- choix d’une sortie précise ;
- suivi automatique de la sortie système par défaut.

### 20.5 Données

- export du profil actif ;
- import d’un profil `.ktm` ;
- import d’une sauvegarde historique JSON.

### 20.6 Découverte

- consultation de la liste noire du profil ;
- réautorisation individuelle d’une vidéo refusée.

### 20.7 Diagnostic

- affichage de la version ;
- ouverture du dossier de données ;
- ouverture du dossier de journaux.

## 21. Raccourcis et interactions

| Action | Raccourci ou geste |
|---|---|
| Arrêter tous les sons | `Ctrl+Shift+S` par défaut, configurable |
| Déclencher ponctuellement le Momentum | `Shift + touche` par défaut, configurable |
| Activer/désactiver Auto-Momentum | Raccourci configurable |
| Activer/désactiver la détection | Raccourci configurable |
| Ouvrir/fermer l’aide | `?` ou `F1` |
| Ouvrir la recherche | `Ctrl/Cmd + F` |
| Annuler | `Ctrl/Cmd + Z` |
| Rétablir | `Ctrl + Y` ou `Cmd+Shift+Z` |
| Tout sélectionner dans la grille | `Ctrl/Cmd + A` |
| Sélection multiple | `Ctrl/Cmd + clic` |
| Sélection d’une plage | `Shift + clic` |
| Suggestion précédente/suivante | Flèches gauche/droite lorsque le panneau est actif |
| Valider une saisie courte | `Entrée` |
| Fermer ou annuler | `Échap` |
| Ajuster un curseur | Molette de la souris au-dessus du curseur |
| Déplacer le Momentum | Clic ou glisser sur la forme d’onde |
| Redimensionner le détail | Glisser la barre de séparation |
| Ajouter des fichiers | Glisser-déposer dans la fenêtre |

## 22. Limites et comportements à connaître

| Élément | Limite ou comportement |
|---|---|
| Pistes par profil | 20 maximum |
| Sons par touche | Pas de limite fonctionnelle imposée dans l’interface |
| Sons simultanés | Un principal par piste, plusieurs pistes en parallèle |
| Historique annuler/rétablir | 50 opérations |
| Cooldown réglable dans l’interface | 0 à 2 000 ms |
| Crossfade | 100 à 2 000 ms |
| Fenêtre d’accord | 20 à 100 ms |
| Téléchargements anticipés de découverte | 3 simultanés au maximum |
| Références utilisées par une recherche de découverte | 15 maximum |
| Résultats conservés par recherche de découverte | 30 maximum |
| Durée des recommandations de découverte | De 30 secondes à 15 minutes |
| Fichiers locaux | Référencés sur place, sauf lorsqu’ils sont importés depuis un `.ktm` |
| Sons des pistes principales | Arrêt et déplacement disponibles, pas de pause/reprise dédiée |
| YouTube et découverte | Connexion Internet nécessaire pour chercher ou télécharger |
| Pavé numérique + Shift | Peut être transformé en touche de navigation par le système |

## 23. Synthèse

KeyToMusic couvre tout le cycle de vie d’une soundboard :

- constitution de bibliothèques audio par profil ;
- récupération de sons locaux ou YouTube ;
- import de playlists ;
- organisation en pistes superposables ;
- assignation simple, combinée ou multi-pistes au clavier ;
- lecture aléatoire, unique, séquentielle ou sans boucle ;
- transitions, volumes, navigation et Momentum ;
- recherche, édition groupée et annulation ;
- découverte de nouveaux sons ;
- export portable, import et récupération des fichiers manquants ;
- utilisation globale sans interrompre la lecture en cours.

Sa fonction centrale est de rendre le pilotage musical immédiat et discret :
l’utilisateur prépare sa configuration une fois, puis contrôle toute son
ambiance sonore au clavier pendant qu’il lit.
