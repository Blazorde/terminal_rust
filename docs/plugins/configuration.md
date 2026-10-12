# Configuration des plugins

Terminal Rust réserve dans le format de configuration v1 un espace stable pour
activer les futurs plugins et conserver leurs options. Ce contrat est pris en
charge dès maintenant ; le chargement et l'exécution de plugins arriveront dans
un ticket ultérieur.

## Format TOML

```toml
[plugins]
enabled = ["git", "docker", "node"]

[plugins.git]
show_branch = true
show_dirty = true

[plugins.docker]
context = "desktop-linux"
```

`plugins.enabled` est la liste ordonnée des plugins à charger. Une liste absente
ou vide désactive tous les plugins. Retirer un identifiant de cette liste suffit
à désactiver le plugin ; sa table reste dans le fichier et ses réglages sont
conservés.

Chaque table `[plugins.<identifiant>]` appartient au plugin correspondant. Le
cœur conserve ses valeurs TOML sans interpréter ni supprimer les clés inconnues.
Une configuration n'est pas obligatoire pour activer un plugin.

## Identifiants

Un identifiant contient entre 1 et 64 caractères ASCII minuscules. Les lettres,
chiffres, points, tirets et tirets bas sont autorisés ; le premier et le dernier
caractère doivent être une lettre ou un chiffre. Les identifiants simples comme
`git` et qualifiés comme `acme.git-tools` sont donc valides.

La même règle s'applique aux valeurs de `plugins.enabled` et aux noms des tables.
Un identifiant ne peut apparaître qu'une fois dans `enabled`. L'ordre est
conservé afin que le futur registre charge les plugins de façon déterministe.

## Comportements d'erreur

| Situation | Comportement |
| --- | --- |
| Section `[plugins]` absente | Configuration valide, aucun plugin activé |
| Identifiant invalide ou dupliqué | Le fichier est rejeté avec le chemin et la propriété concernés |
| Valeur `[plugins.<id>]` qui n'est pas une table | Le TOML est rejeté avec le chemin du fichier |
| Table d'un plugin désactivé | Réglages conservés, plugin non chargé |
| Plugin activé sans table | Chargement autorisé avec une configuration vide |
| Plugin activé mais non installé | Le schéma accepte le fichier ; le futur registre signalera le plugin indisponible et continuera sans l'exécuter |
| Clé inconnue dans une table de plugin | Conservée et transmise au plugin, qui en assure la validation |

Comme pour toute erreur de configuration globale, le shell affiche un diagnostic
puis démarre avec ses valeurs sûres par défaut. L'absence d'un plugin ou d'un
outil externe reste une capacité indisponible, pas une erreur fatale du shell.

## Compatibilité

Le format n'encode aucun chemin de plateforme et fonctionne de façon identique
sur Windows, macOS et Linux. Les chemins éventuellement utilisés dans les
options d'un plugin restent des chaînes TOML ; le plugin doit les convertir avec
les API natives de chemins et documenter leur portée.

Les anciens fichiers v1 sans section `[plugins]` restent valides. Le fichier
canonique [`config/default.toml`](../../config/default.toml) conserve une liste
vide afin qu'une installation neuve n'exécute jamais une extension implicitement.
