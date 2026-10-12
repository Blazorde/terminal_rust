# Template de plugin Terminal Rust

Ce dossier est un modèle minimal copiable. Il contient un manifeste reconnu par
le registre, une commande `hello` et un test unitaire. Le code utilise seulement
les DTO du SDK ; il ne choisit pas le futur transport entre l'hôte et le plugin.

## Copier le modèle

1. Copier le dossier hors de `plugins/example-plugin` ou le renommer.
2. Modifier `name` dans `Cargo.toml`.
3. Remplacer `id`, `name`, `version`, `description` et `authors` dans
   `terminal-rust-plugin.toml`.
4. Adapter l'identifiant `example.hello/hello` et la réponse dans `src/lib.rs`.
5. Garder uniquement les permissions réellement nécessaires.

L'identifiant du plugin est stable ; ne le changez pas après publication. Le
couple version du manifeste/version du crate doit rester identique.

## Compiler et tester

Depuis la racine du dépôt :

```bash
cargo check -p terminal-rust-example-plugin --all-targets
cargo clippy -p terminal-rust-example-plugin --all-targets -- -D warnings
cargo test -p terminal-rust-example-plugin --all-targets
```

Le test appelle directement l'implémentation et vérifie la réponse typée. Le
manifeste est aussi parsé par les tests du cœur. Le template n'est pas chargé
dynamiquement : le choix du runtime isolé reste volontairement séparé.

## Tester l'installation locale

Créer l'arborescence suivante dans le [répertoire local des
plugins](../../docs/plugins/registry.md), puis y copier le manifeste :

```text
terminal-rust/plugins/example.hello/0.1.0/terminal-rust-plugin.toml
```

Ajouter `example.hello` à `plugins.enabled`, puis lancer :

```bash
terminal_rust plugins list
```

Le plugin doit apparaître `enabled`. Sa commande ne devient exécutable qu'après
l'arrivée du runtime ; le registre n'exécute jamais une bibliothèque copiée.

## Checklist avant partage

- identifiant et SemVer valides ;
- manifeste et crate à la même version ;
- permissions minimales ;
- aucune donnée sensible dans les exemples ou tests ;
- formatage, Clippy strict et tests réussis ;
- erreurs utilisateur courtes et actionnables ;
- comportement documenté sur Windows, macOS et Linux.
