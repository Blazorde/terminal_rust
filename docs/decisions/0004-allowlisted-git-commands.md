# ADR 0004 - Limiter les commandes Git intégrées à une liste blanche

- Statut : Acceptée
- Date : 2026-10-09

## Contexte

Le mini-shell doit aider les développeurs à consulter rapidement l'état d'un
dépôt, ses branches et son historique. Un passthrough Git complet exposerait
aussi des opérations distantes ou destructrices sans ajouter de valeur
pédagogique au shell.

## Décision

La commande interne `git` délègue uniquement `status`, `branch` et `log` à
l'exécutable Git installé. Elle transmet leurs options de consultation,
conserve stdout, stderr et le code de sortie, et force `--no-pager` pour éviter
de bloquer le shell interactif. Les arguments mutateurs de `branch` et l'option
`log --output` sont refusés avant le lancement.

Le processus est lancé avec `git -C <dossier courant>` afin de garder le même
comportement sous Windows, macOS et Linux. Toute autre sous-commande est
refusée avant le lancement du processus.

## Conséquences

- Les consultations Git courantes sont disponibles sans réimplémenter Git.
- `push`, `reset`, `clean` et les autres opérations non autorisées ne peuvent
  pas être lancées par cette commande interne.
- Git reste une dépendance optionnelle : son absence produit une erreur claire
  sans fermer le shell.
- Toute nouvelle sous-commande devra être ajoutée explicitement avec ses tests
  et une justification de sécurité.
