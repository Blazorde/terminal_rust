# Registre local des plugins

Le registre local inventorie des manifestes, sans charger de bibliothèque ni
lancer de processus. L'organisation canonique est :

```text
<data-dir>/terminal-rust/plugins/<plugin-id>/<version>/terminal-rust-plugin.toml
```

| Système | `<data-dir>` |
| --- | --- |
| Windows | `%LOCALAPPDATA%` |
| macOS | `$HOME/Library/Application Support` |
| Linux | `$XDG_DATA_HOME`, sinon `$HOME/.local/share` |

`plugins list` affiche chaque version dans un ordre déterministe avec son état :
`enabled`, `disabled`, `incompatible` ou `error`. L'activation vient de
`plugins.enabled` dans la configuration utilisateur. Un dossier absent signifie
simplement qu'aucun plugin n'est installé. Une erreur de lecture, en revanche,
nomme le dossier concerné.

La découverte est limitée à l'arborescence attendue, ignore les liens
symboliques et ne suit jamais un chemin fourni par un manifeste. Ces contraintes
réduisent les traversées imprévues et gardent le comportement identique sur les
trois plateformes.

Le registre ne résout pas encore les doublons, n'active aucune contribution et
n'exécute aucun plugin. Les tickets de commandes et de segments consommeront
uniquement les entrées compatibles et activées lors des étapes suivantes.
