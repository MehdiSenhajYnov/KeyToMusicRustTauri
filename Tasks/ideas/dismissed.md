# Dismissed Ideas

| # | Idea | Summary | Date |
|---|------|---------|------|
| 1 | **Fondu d'entrée/sortie par son** | Fade-in/fade-out configurable par son, indépendant du crossfade entre tracks | 2026-02-02 |
| 2 | **Visualisation clavier physique** | Vue QWERTY/AZERTY interactive remplaçant/complétant le KeyGrid avec couleurs par track | 2026-02-02 |
| 3 | **Preview discovery avec timeline** | Seek en temps réel sur la waveform du carousel discovery + curseur de lecture animé | 2026-02-02 |
| 4 | **Seeds discovery manuels** | Permettre d'ajouter des URLs YouTube comme seeds custom pour guider la discovery | 2026-02-02 |
| 5 | **Mute par track avec raccourci** | Bouton mute par track + raccourci global optionnel (Ctrl+1, etc.) | 2026-02-02 |
| 6 | **Tags et notes sur les profils** | Champs tags[] et notes sur Profile avec badges colorés et filtre par tag | 2026-02-02 |
| 7 | **Couleurs par Track dans le KeyGrid** | Couleur unique par track (auto ou picker), visible comme fond/bordure des touches dans le grid | 2026-02-02 |
| 8 | **Sons récemment joués dans la sidebar** | Ring buffer des 5-10 derniers sons joués sous NowPlaying, re-jouables en un clic | 2026-02-02 |
| 9 | **Virtualisation du KeyGrid** | react-window pour ne rendre que les touches visibles, optimisation pour profils 80+ bindings | 2026-02-02 |
| 10 | **Double-tap pour rejouer depuis le début** | Tap x2 rapide = lecture position 0, simple tap = momentum normal, toggle dans settings | 2026-02-02 |
| 11 | **Dupliquer binding vers autre touche** | Bouton dans SoundDetails copiant sons/volume/loop/momentum vers nouvelle touche | 2026-02-02 |
| 12 | **Templates de profil** | Presets (Standard, Musique, Minimal) avec tracks pré-créés à la création d'un profil | 2026-02-02 |
| 13 | **Equalizer par Track avec Presets** | EQ 3-bandes (basses, mediums, aigus) par track avec presets (Voice, Music, SFX), appliqué via rodio Filter trait et persisted dans profile | 2026-02-02 |
| 14 | **Historique des sons joués** | Ring buffer des 10 derniers sons sous NowPlaying avec bouton re-play rapide, géré par playHistoryStore, persist en session | 2026-02-02 |
| 15 | **Virtualisation KeyGrid (react-window)** | Remplacer le rendu flat par FixedSizeGrid virtualisé pour profils 100+ bindings, améliore scroll et filtre perfs | 2026-02-02 |
| 16 | **Auto-normalisation du volume par son** | Analyse LUFS + ajustement auto du Sound.volume pour uniformiser loudness, bouton "Normalize All" dans Settings | 2026-02-02 |
| 17 | **Groupes et folders dans KeyGrid** | Organisation hiérarchique des bindings avec headers collapsibles, drag & drop, persistés dans Profile.groups | 2026-02-02 |
| 18 | **Backup automatique des profils** | Snapshot toutes les 30min dans data/backups/, garde 20 derniers, restore UI avec diff preview | 2026-02-02 |
| 19 | **Pitch shifter par son** | Champ pitch (-12 to +12 semitones) par Sound via rubato crate, slider dans SoundDetails avec preview temps réel | 2026-02-02 |
| 20 | **Import depuis autres soundboards** | Support Resanance/Soundux/EXP Soundboard pour migration facile, parse format externe → nouveau profil KeyToMusic | 2026-02-02 |
