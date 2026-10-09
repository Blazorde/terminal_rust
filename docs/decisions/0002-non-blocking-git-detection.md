# ADR-0002 — Détection Git externe et non bloquante

- Statut : Acceptée
- Date : 2026-10-09
- Décideurs : équipe Terminal Rust

## Contexte

Le futur prompt doit reconnaître un dépôt Git sur Windows, macOS et Linux. Git
peut être absent, le dossier peut être hors dépôt et les structures `.git`
peuvent être des dossiers ou des fichiers selon les worktrees et submodules.

## Décision

La détection appelle directement :

```text
git -C <path> rev-parse --is-inside-work-tree
```

Le processus est lancé sans shell. La fonction retourne un booléen et considère
l'exécutable absent, un chemin invalide, un code d'échec ou une sortie inattendue
comme `false`. `stderr` est masqué car ces cas sont normaux pour une décoration
optionnelle.

## Options considérées

- Chercher un dossier `.git` en remontant les parents : rapide mais incorrect
  pour certaines configurations Git.
- Utiliser une bibliothèque Git : API riche mais dépendance lourde pour cette
  première brique.
- Appeler Git : cohérent avec le comportement de l'outil installé et portable.

## Conséquences

### Positives

- Compatible avec les worktrees et la configuration Git réelle.
- Le shell reste utilisable sans Git.
- Aucun parsing de chemin spécifique à un OS.

### Négatives

- Création d'un processus à chaque détection.
- La branche et le statut nécessiteront d'autres appels ou une commande groupée.
- Un cache et une limite de durée seront nécessaires avant un prompt riche.

## Validation

Les tests couvrent une sortie positive, une sortie invalide, un exécutable
absent, un dossier hors dépôt et un dépôt temporaire initialisé lorsque Git est
disponible.
