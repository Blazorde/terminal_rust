# Personnalisation du prompt

Terminal Rust vise un prompt composé de segments, inspiré d'Oh My Posh mais
volontairement plus petit et déclaratif. Le chargeur TOML et la composition
dynamique appliquent maintenant ce contrat.

## Séparation des responsabilités

Le système comporte trois couches :

1. les fournisseurs collectent des données comme le dossier courant ou l'état
   Git ;
2. la configuration choisit les segments, leur ordre et leurs options ;
3. le thème transforme ces données en texte, couleurs, icônes et séparateurs.

Un thème ne lance jamais de commande. Un segment indisponible s'omet sans rendre
le shell inutilisable.

## Emplacements et priorité

Le fichier intégré `config/default.toml` documente les valeurs par défaut. Le
chargeur cherche une configuration utilisateur dans le dossier
de configuration fourni par le système d'exploitation, sans supposer que
`HOME` existe :

- Windows : `%APPDATA%\terminal-rust\config.toml` ;
- macOS : `$HOME/Library/Application Support/terminal-rust/config.toml` ;
- Linux : `$XDG_CONFIG_HOME/terminal-rust/config.toml`, ou
  `$HOME/.config/terminal-rust/config.toml` si `XDG_CONFIG_HOME` est absent.

La variable `TERMINAL_RUST_CONFIG` peut sélectionner explicitement un autre
fichier. Si aucun emplacement n'est disponible, les valeurs intégrées restent
actives.

## Format utilisateur

Le format TOML est versionné. `version = 1` est obligatoire afin de refuser
proprement une configuration future incompatible.

```toml
version = 1
theme = "default"

[prompt]
segments = ["path", "git", "symbol"]
icons = "auto"
separator = "  "
newline = false

[segments.path]
style = "full"

[segments.git]
show_status = true

[segments.symbol]
value = ">"
```

Options personnalisables :

| Clé | Type | Rôle | Défaut |
| --- | --- | --- | --- |
| `version` | entier | Version du schéma | obligatoire |
| `theme` | texte | Thème intégré ou chemin TOML | `default` |
| `prompt.segments` | liste | Segments et ordre d'affichage | path, git, symbol |
| `prompt.icons` | `auto`, `always`, `never` | Politique d'icônes | `auto` |
| `prompt.separator` | texte | Espace entre segments | deux espaces |
| `prompt.newline` | booléen | Symbole final sur une nouvelle ligne | `false` |

Les clés inconnues produiront un avertissement utile. Une valeur invalide fera
revenir à la valeur par défaut concernée, sans empêcher le démarrage du shell.

## Segments v1

| Identifiant | Données | Options | Repli |
| --- | --- | --- | --- |
| `path` | dossier courant | `style = full|folder` | chemin fourni par `Path::display` |
| `git` | branche, état modifié | `show_status` | segment omis hors dépôt ou sans Git |
| `symbol` | symbole final | `value` | `>` |

L'ordre de `prompt.segments` est conservé exactement. Un segment absent de la
liste n'est ni collecté ni affiché. Les doublons et identifiants inconnus sont
refusés avec un diagnostic clair. `prompt.separator` sépare les segments visibles
et `prompt.newline = true` place le symbole sur une nouvelle ligne lorsqu'il est
précédé d'un autre segment.

Les futurs segments doivent avoir un identifiant stable, des données typées et
un coût borné. Les commandes externes personnalisées et les expressions ne font
pas partie du format v1.

## Capacités et mode sans icônes

`icons = "never"` force un rendu ASCII sans Nerd Font. `always` utilise les
icônes du thème. `auto` ne les active que lorsque le détecteur de capacités les
juge sûres ; en cas de doute, il choisit le mode sans icônes. Le thème doit donc
fournir un `fallback` ASCII pour chaque icône.

La même politique peut être surchargée avec
`TERMINAL_RUST_ICONS=auto|always|never`. En mode `auto`, définir
`TERMINAL_RUST_NERD_FONT=1` confirme explicitement qu'une Nerd Font est active.

La couleur suit la même règle de dégradation : sortie non interactive,
`NO_COLOR` ou `TERM=dumb` donnent un rendu sans ANSI. Le chemin, la branche et le
symbole restent lisibles dans tous les modes.

## Validation et sécurité

- Résoudre les chemins avec `Path` et `PathBuf`.
- Limiter la taille des chaînes et rejeter les caractères de contrôle dans les
  symboles, icônes et séparateurs.
- Ne jamais interpréter une valeur TOML comme une commande.
- Signaler le fichier et la clé d'une erreur, puis utiliser un repli sûr.
- Conserver un thème intégré pour démarrer même si les fichiers utilisateur
  sont absents ou illisibles.

Le [format des thèmes](themes.md) décrit la partie visuelle de ce contrat.
