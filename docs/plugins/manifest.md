# Manifeste de plugin

Chaque version installée fournit un fichier `terminal-rust-plugin.toml`. Le
registre lit ce fichier sans exécuter le plugin. Le format initial est strict :
une clé inconnue, une permission non prise en charge ou un champ obligatoire
absent produit un diagnostic lié au chemin du manifeste.

```toml
schema_version = 1
id = "acme.git-tools"
name = "Git tools"
version = "0.1.0"
description = "Ajoute des diagnostics Git sans modifier le dépôt"
host_api = "^1.0"
authors = ["Acme"]
permissions = ["command.register", "output.write", "prompt.segment.register"]

[[commands]]
name = "git-clean-check"
summary = "Vérifie si le dépôt est propre"

[[prompt_segments]]
name = "git-review"
summary = "État de la revue en cours"
timeout_ms = 20
cache_ttl_ms = 5000
```

`version` suit SemVer. `host_api` est une exigence SemVer comparée à l'API hôte
`1.0.0`. `id` suit les règles de la [configuration](configuration.md). Un nom de
commande contient uniquement des minuscules ASCII, chiffres, `-` ou `_`.

Les permissions acceptées en v1 sont `context.cwd.read`, `context.env.read`,
`config.read`, `command.register`, `completion.register`, `output.write` et
`prompt.segment.register`. Elles déclarent une intention ; ce ticket ne charge
ni n'exécute aucun code externe.

Une contribution exige la permission correspondante. Les segments déclarent un
budget de 1 à 100 ms et un cache maximal de 300 000 ms. Les valeurs omises sont
respectivement 25 ms et aucun cache.

Un manifeste invalide reste visible avec l'état `error`. Une version dont
`host_api` n'accepte pas l'hôte reste visible avec l'état `incompatible`.
