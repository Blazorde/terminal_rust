# Stratégie de tests

## Pyramide actuelle

Le projet privilégie des tests unitaires rapides au plus près des modules. Les
tests d'intégration CLI seront ajoutés dans `tests/` lorsque les comportements de
sortie seront assez stables.

## Suite locale

Exécuter avant chaque PR :

```bash
cargo fmt --all -- --check
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
```

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

## CI

La CI actuelle couvre Ubuntu et Windows. Une matrice macOS doit être ajoutée
avant de déclarer le moteur de prompt complet multiplateforme. Un échec sur une
plateforme bloque la fusion.
