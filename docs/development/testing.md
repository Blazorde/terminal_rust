# Stratégie de tests

## Pyramide actuelle

Le projet privilégie des tests unitaires rapides au plus près des modules. Les
tests d'intégration dans `tests/cli_tests.rs` lancent aussi le binaire comme un
utilisateur pour vérifier ses sorties et ses codes de retour.

## Suite locale

Exécuter avant chaque PR :

```bash
cargo fmt --all -- --check
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
```

`cargo test` compile le binaire puis exécute les tests unitaires et les tests
d'intégration CLI. Ces derniers utilisent des dossiers temporaires uniques,
notamment avec des espaces dans leur nom, et ne dépendent pas du dossier
personnel de l'utilisateur.

## Cas à couvrir

### Commandes de fichiers

- chemin valide ;
- argument manquant ;
- fichier contre dossier ;
- chemin contenant des espaces ;
- opération refusée ou impossible.

### Parser et dispatcher

- espaces multiples et guillemets ;
- guillemet non fermé ;
- commande inconnue ;
- nombre d'arguments invalide ;
- commande visible dans la complétion et l'aide.

### Intégrations externes

- outil disponible et réponse valide ;
- outil absent ;
- code de sortie en erreur ;
- sortie vide, inattendue ou non UTF-8 ;
- exécution sans passer par un shell.

## Tests dépendants de l'environnement

Un test peut ignorer une vérification complémentaire si Git n'est pas installé,
mais les branches « outil absent » et « parsing de sortie » doivent rester
testables sans Git. Les dossiers temporaires doivent être uniques et supprimés à
la fin du test.

## Tests CLI

La suite d'intégration vérifie actuellement :

- `terminal_rust pwd` et son dossier de travail ;
- `terminal_rust ls` avec un fichier et un dossier de test ;
- une commande inconnue, sa sortie d'erreur et son code de retour ;
- une commande sans argument obligatoire, sa sortie d'erreur et son code de
  retour.

Les assertions tolèrent les différences de fins de ligne entre plateformes et
comparent les chemins après résolution par la bibliothèque standard Rust.

## CI

La CI couvre Ubuntu, Windows et macOS. Chaque système exécute le formatage, la
compilation, Clippy, les tests unitaires et les tests d'intégration CLI. Un
échec sur une plateforme bloque la fusion.
