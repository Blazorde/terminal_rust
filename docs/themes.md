# Format des thèmes

Un thème contrôle l'apparence des segments sans choisir les données collectées.
Le format TOML est versionné indépendamment de l'implémentation du renderer.

## Structure v1

```toml
version = 1
name = "default"

[palette]
path = "cyan"
git_clean = "green"
git_dirty = "yellow"
symbol = "white"

[segments.path]
foreground = "path"
icon = "folder"
fallback = ""

[segments.git]
foreground_clean = "git_clean"
foreground_dirty = "git_dirty"
icon = "git_branch"
fallback = "git:"

[segments.symbol]
foreground = "symbol"
value = ">"
```

Les couleurs peuvent référencer un nom de la palette. La liste exacte des
couleurs et la traduction ANSI seront définies avec le renderer ; un nom inconnu
doit produire un avertissement puis revenir à la couleur du terminal.

## Règles

- `version` et `name` sont obligatoires.
- Une table de segment ne peut styliser qu'un identifiant connu.
- `icon` désigne une icône symbolique, jamais un glyphe codé en dur dans la
  configuration utilisateur.
- `fallback` est du texte ASCII affichable sans Nerd Font ; une chaîne vide
  signifie « pas de préfixe ».
- Une couleur absente conserve la couleur du terminal.
- Le renderer réinitialise son style après chaque segment pour éviter les fuites
  de couleur dans la ligne de commande.

## Résolution

`theme = "default"` utilise le thème intégré. Une valeur qui se termine par
`.toml` sera traitée comme un chemin relatif au fichier de configuration, ou
comme un chemin absolu natif. Un thème manquant, illisible ou incompatible
retombe sur `default` et produit un diagnostic non fatal.

## Thème par défaut

`config/themes/default.toml` constitue l'exemple canonique. Il privilégie des
couleurs ANSI simples, un contraste lisible et des fallbacks ASCII. Le rendu
minimal attendu sans ANSI ni icône reste :

```text
C:\workspace\terminal_rust  git:main* >
```

Le thème ne suppose ni police particulière, ni palette 24 bits, ni terminal
Unix. Les séparateurs Powerline et les palettes avancées pourront enrichir le
schéma sans changer les fournisseurs de données.
