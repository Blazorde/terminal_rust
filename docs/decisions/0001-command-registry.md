# ADR-0001 — Registre central des commandes internes

- Statut : Acceptée
- Date : 2026-10-09
- Décideurs : équipe Terminal Rust

## Contexte

Le shell doit ajouter régulièrement des commandes tout en alimentant le
dispatcher et l'autocomplétion. Des listes indépendantes finiraient par diverger.

## Décision

Les commandes internes sont déclarées dans la table `COMMANDS` de
`src/app/command.rs`. Chaque entrée associe un nom, une syntaxe, un résumé, une
limite d'arguments et un handler. `command_names()` expose cette source à
l'autocomplétion et `command_help()` génère l'aide depuis les mêmes entrées.

La logique métier reste dans `src/commands/`; les handlers du registre adaptent
seulement les arguments, l'état du shell et les erreurs.

## Options considérées

- Grand `match` dans le runner : simple au départ mais mélange le cycle de vie et
  les commandes.
- Registres séparés pour exécution, aide et complétion : explicites mais faciles
  à désynchroniser.
- Système de plugins dynamique : prématuré pour le MVP.

## Conséquences

### Positives

- Le runner reste stable lors de l'ajout d'une commande.
- La complétion réutilise les noms réellement exécutables.
- La validation commune des arguments est centralisée.

### Négatives

- Les signatures de handlers doivent s'adapter au type commun.
- Les contributions externes devront passer par un adaptateur isolé avant de
  rejoindre cette vue du registre.

## Validation

Une nouvelle commande doit être exécutable, proposée par la complétion et
documentée par `help`. Des tests de complétion protègent les commandes ajoutées.
