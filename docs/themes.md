# Format des thèmes

Un thème contrôle l'apparence des segments sans choisir les données collectées.
Le format TOML est versionné indépendamment de l'implémentation du renderer.

## Structure v1

```toml
version = 1
name = "default"
icons = "auto"

[palette]
primary = "cyan"
secondary = "magenta"
success = "green"
warning = "yellow"
error = "red"
muted = "white"

[segments.path]
foreground = "primary"
icon = "folder"
fallback = ""

[segments.git]
foreground_clean = "success"
foreground_dirty = "warning"
icon = "git_branch"
fallback = "git:"

[segments.symbol]
foreground = "secondary"
value = ">"
```

Les segments référencent uniquement un rôle sémantique de la palette :
`primary`, `secondary`, `success`, `warning`, `error` ou `muted`. Le thème mappe
ensuite ces rôles vers l'une des huit couleurs ANSI portables : `black`, `red`,
`green`, `yellow`, `blue`, `magenta`, `cyan` ou `white`. Un nom inconnu revient à
la couleur du terminal ; le futur chargeur de thème ajoutera aussi un diagnostic
non fatal.

La palette intégrée utilise cyan pour le chemin (`primary`), magenta pour le
symbole (`secondary`), vert pour un dépôt propre (`success`) et jaune pour un
dépôt modifié (`warning`). Modifier ces associations ne demande aucune
modification des modules qui collectent le chemin ou l'état Git.

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

## Choisir un preset

Trois thèmes sont intégrés et ne nécessitent aucun fichier supplémentaire :

| Valeur | Usage | Icônes par défaut |
| --- | --- | --- |
| `default` | Palette équilibrée pour un usage général | détection automatique |
| `minimal` | Peu de couleurs, compatible avec toutes les polices | désactivées |
| `git-focus` | État Git plus contrasté, notamment lorsqu'il est modifié | détection automatique |
| `powerline` | Séparateurs triangulaires colorés entre les segments | détection automatique |

Choisissez le preset dans le fichier de configuration utilisateur :

```toml
version = 1
theme = "minimal"
```

Une valeur inconnue produit un diagnostic non fatal et le shell utilise le
thème `default`. `prompt.icons = "always"` ou `"never"` garde la priorité sur
la préférence du thème. Le chargement d'un thème depuis un chemin personnalisé
n'est pas encore pris en charge.

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

`config/themes/minimal.toml` force les icônes à `never` et réduit les rôles
principaux au blanc. Il reste donc lisible sans Nerd Font et avec les capacités
ANSI de base. `config/themes/git-focus.toml` conserve les fallbacks ASCII mais
utilise le rouge pour rendre immédiatement visible un dépôt modifié.

## Séparateurs Powerline

Le preset `powerline` utilise le glyphe `U+E0B0` entre les segments et lui
applique la couleur du segment précédent. Son activation se fait comme pour les
autres presets :

```toml
theme = "powerline"
```

Le glyphe enrichi nécessite une Nerd Font et la même détection prudente que les
icônes. Définissez `TERMINAL_RUST_NERD_FONT=1` (`true` et `yes` sont aussi
acceptés) lorsque la police du terminal est compatible. Sans confirmation, sans
terminal interactif, ou avec `prompt.icons = "never"`, le séparateur devient
automatiquement ` | `. Le prompt reste donc lisible sous Windows, macOS et
Linux avec une police standard. `prompt.separator` continue de contrôler les
thèmes qui n'ont pas de séparateur dédié.

## Icônes et repli ASCII

Le registre `render::icons` centralise les glyphes `folder`, `git_branch`,
`error`, `user` et `time`. Les thèmes référencent ces noms symboliques au lieu
de copier des caractères Unicode. Une Nerd Font, par exemple MesloLGS NF, est
nécessaire pour le rendu enrichi.

Avec `icons = "always"`, le prompt affiche les glyphes du registre. Avec
`icons = "never"`, il concatène le `fallback` ASCII du thème à la valeur. Le
mode `auto` reste sans icônes lorsqu'il ne peut pas confirmer la présence d'une
Nerd Font. Une icône inconnue utilise aussi son fallback : le prompt demeure
donc lisible dans tous les terminaux Windows, macOS et Linux.
