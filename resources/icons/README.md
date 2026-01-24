# Application Icons

Ce dossier doit contenir les icônes de l'application dans les formats suivants :

- **32x32.png** : Icône 32x32 pixels
- **128x128.png** : Icône 128x128 pixels
- **128x128@2x.png** : Icône 128x128 pixels haute résolution
- **icon.icns** : Icône macOS
- **icon.ico** : Icône Windows

Ces icônes seront utilisées par Tauri pour créer l'application sur différentes plateformes.

## Génération automatique

Vous pouvez utiliser un outil comme [Tauri Icon Generator](https://tauri.app/v1/guides/features/icons/) pour générer tous les formats à partir d'une seule image PNG de haute résolution (minimum 512x512 pixels).

```bash
npm run tauri icon path/to/icon.png
```
