# Guide de contribution

## Préparer l'environnement

Installer Rust avec `rustup`, puis vérifier :

```bash
rustc --version
cargo --version
git --version
```

Git améliore le futur prompt mais ne doit pas être requis pour exécuter les
commandes de base du shell.

## Choisir une branche

Partir de `develop` et choisir le préfixe correspondant au changement :

```bash
git switch develop
git pull --ff-only origin develop
git switch -c feat/short-description
```

| Préfixe | Usage |
| --- | --- |
| `feat/` | Nouvelle capacité utilisateur |
| `fix/` | Correction de comportement |
| `chore/` | Documentation, CI, maintenance |
| `refractor/` | Refactor sans changement fonctionnel |
| `hotfix/` | Correction urgente issue de production |

## Ajouter une commande

1. Créer le module dans `src/commands/`.
2. Ajouter un type dans `src/errors/` si plusieurs échecs métier sont possibles.
3. Exporter les modules nécessaires.
4. Enregistrer la commande dans `app/command.rs`.
5. Ajouter l'aide et vérifier la complétion.
6. Tester le succès, les arguments invalides et les erreurs de fichiers.
7. Documenter la commande dans le README.

## Changer l'architecture

Créer un ADR si le changement :

- introduit une dépendance structurante ;
- modifie les frontières de modules ;
- choisit une stratégie multiplateforme ;
- affecte plusieurs tickets futurs ;
- abandonne une décision déjà documentée.

## Pull request

La PR cible `develop` et contient :

- le problème ou ticket traité ;
- les choix importants ;
- les validations exécutées ;
- les limites ou suites prévues.

Les règles GitHub complètes sont décrites dans
[`docs/workflows/README.md`](../workflows/README.md).
