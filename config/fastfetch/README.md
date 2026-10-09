# Preset Fastfetch Terminal Rust

Ce preset offre une vue compacte et colorée du système sur Windows, macOS et
Linux. Il utilise uniquement des modules Fastfetch standards et ne dépend pas
d'une Nerd Font.

## Utilisation depuis le dépôt

```bash
fastfetch --config config/fastfetch/config.jsonc
```

Fastfetch doit être installé séparément. Le projet ne l'installe pas et la
commande interne `dashboard` fonctionne sans lui.

## Installation comme configuration par défaut

Créer le dossier de configuration Fastfetch, sauvegarder une éventuelle
configuration existante, puis copier `config.jsonc` vers :

```text
~/.config/fastfetch/config.jsonc
```

Ce chemin est reconnu par Fastfetch sur Windows, macOS et Linux. La
configuration suit le [schéma officiel Fastfetch](https://github.com/fastfetch-cli/fastfetch/blob/dev/doc/json_schema.json).

## Personnalisation

- Modifier `keyColor` pour changer une catégorie.
- Retirer `gpu` ou `battery` si la machine n'expose pas ces informations.
- Remplacer `logo.type` par `auto` pour utiliser le logo complet du système.
- Conserver le champ `$schema` pour la validation et l'autocomplétion dans
  l'éditeur.
