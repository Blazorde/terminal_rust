# Configuration utilisateur

Terminal Rust charge un fichier TOML optionnel au démarrage interactif. Son
absence n'est pas une erreur : les valeurs intégrées de `config/default.toml`
sont utilisées.

## Emplacements

- Windows : `%APPDATA%\terminal-rust\config.toml`
- macOS : `$HOME/Library/Application Support/terminal-rust/config.toml`
- Linux : `$XDG_CONFIG_HOME/terminal-rust/config.toml`, ou
  `$HOME/.config/terminal-rust/config.toml`

`TERMINAL_RUST_CONFIG` peut désigner explicitement un autre fichier avec un
chemin natif absolu ou relatif. Contrairement à la recherche automatique, un
chemin explicite absent produit un diagnostic.

## Format v1

Le fichier [d'exemple](../config/default.toml) documente toutes les options. Les
sections ou clés inconnues, un TOML invalide, une version différente de `1`, un
segment inconnu ou des caractères de contrôle produisent une erreur indiquant
le fichier concerné. Le shell affiche le diagnostic puis démarre avec ses
valeurs par défaut.

Les champs absents reçoivent leur valeur par défaut. La politique
`prompt.icons`, le choix des segments, leur ordre, le séparateur, le mode nouvelle
ligne, le style du chemin, l'affichage de l'état Git et le symbole final sont
appliqués au prompt.

`theme` accepte les presets intégrés `default`, `minimal`, `git-focus` et
`powerline`. Le
guide [des thèmes](themes.md) décrit leur rendu et la priorité des options
d'icônes.

La section `[plugins]` réserve la configuration des extensions. Les plugins
restent inactifs tant que leur identifiant n'apparaît pas dans `enabled`. Le
guide [de configuration des plugins](plugins/configuration.md) définit le format
et les comportements d'erreur avant l'arrivée du runtime d'extensions.

## Exemple minimal

```toml
version = 1

[prompt]
icons = "never"
```
