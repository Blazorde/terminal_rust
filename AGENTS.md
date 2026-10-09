# Guide de contribution automatisée

Ce fichier s'applique à tout le dépôt.

## Objectif du projet

Terminal Rust est un mini-shell pédagogique, fiable et multiplateforme. Toute
évolution doit rester compatible avec Windows, macOS et Linux. Le futur moteur
de prompt s'inspire d'Oh My Posh : segments indépendants, configuration lisible
et dégradation propre lorsque Git ou une capacité du terminal manque.

## Source de vérité

- Le ticket Notion définit le périmètre fonctionnel et les critères
  d'acceptation.
- `docs/architecture/` décrit les frontières techniques actuelles et cibles.
- `docs/decisions/` contient les décisions structurantes et leurs compromis.
- Le code et les tests priment si la documentation décrit un état ancien ; la
  documentation doit alors être corrigée dans la même PR.

## Méthode de travail

1. Partir de `develop` à jour.
2. Utiliser une branche `feat/*`, `fix/*`, `chore/*`, `refractor/*` ou
   `hotfix/*` selon la nature du changement.
3. Limiter une branche à un ensemble cohérent de tickets. Ne grouper plusieurs
   tickets que s'ils partagent la même frontière technique et se valident
   ensemble.
4. Préserver les changements locaux qui ne concernent pas le ticket.
5. Ajouter ou adapter les tests avec le code.
6. Mettre à jour le README, les guides ou un ADR lorsqu'un comportement public
   ou une décision d'architecture change.
7. Ouvrir la PR vers `develop` et reporter son lien dans le ticket.

## Règles Rust

- Garder `runner` concentré sur le cycle de vie du shell.
- Enregistrer les commandes internes dans la table de `app/command.rs`.
- Isoler la logique métier dans `commands/`, `git/` ou un module spécialisé.
- Centraliser l'affichage dans `render` afin de faciliter les tests et les
  adaptations de terminal.
- Préférer des erreurs typées et des messages utiles à des `unwrap` ou `panic!`.
- Traiter l'absence d'un outil externe comme une capacité indisponible lorsque
  ce n'est pas une erreur fatale.
- Utiliser `Path` et `PathBuf`, jamais une concaténation manuelle avec `/` ou
  `\\`.
- Ne pas dépendre d'une commande Unix ou PowerShell pour une opération que la
  bibliothèque standard Rust sait réaliser.

## Compatibilité

- Ne pas supposer que `HOME`, Git, une Nerd Font ou les séquences ANSI avancées
  sont disponibles.
- Tester les chemins contenant des espaces et les fins de ligne `LF`/`CRLF`
  lorsque le changement les manipule.
- Toute dépendance spécifique à un OS doit rester derrière `cfg(...)` et offrir
  un comportement de repli documenté.

## Validation obligatoire

```bash
cargo fmt --all -- --check
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
```

Documenter explicitement toute vérification impossible dans l'environnement
local. Ne jamais déclarer un ticket terminé sans preuve correspondant à ses
critères d'acceptation.

## Git

Les commits suivent Conventional Commits, en anglais :

```text
feat(git): expose repository status
fix(prompt): handle missing home directory
chore(docs): document architecture decisions
```

Les détails de branches, PR et protections sont dans
`docs/workflows/README.md`.
