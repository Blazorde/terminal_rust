# Workflow ticket → pull request

Ce guide définit le déroulé attendu lorsqu'un contributeur prend un ticket en
charge de manière autonome.

## 1. Qualifier le ticket

- Lire l'objectif, les fichiers suggérés et tous les critères d'acceptation.
- Vérifier l'état réel de `develop` : le ticket peut déjà être partiellement ou
  totalement implémenté.
- Repérer les dépendances et les tickets voisins.
- Poser une question uniquement si deux réponses changent réellement le produit
  ou la compatibilité.

## 2. Définir le lot

Un ticket correspond normalement à une branche. Plusieurs tickets peuvent être
groupés si :

- ils touchent la même frontière technique ;
- aucun ne peut être validé utilement sans l'autre ;
- la PR reste facile à relire et à annuler.

Ne pas mélanger documentation générale, refactor non requis et feature métier
dans une même branche.

## 3. Implémenter

- Créer la branche depuis `develop` à jour.
- Ajouter les tests en même temps que le comportement.
- Préserver la compatibilité Windows, macOS et Linux.
- Mettre à jour la documentation utilisateur et technique concernée.
- Ajouter un ADR pour une décision qui engage les tickets suivants.

## 4. Vérifier

Exécuter la suite définie dans
[`docs/development/testing.md`](../development/testing.md), puis relire le diff :

- aucun secret ni fichier généré ;
- aucune modification hors périmètre ;
- messages d'erreur cohérents ;
- documentation et code alignés.

## 5. Livrer

- Commit Conventional Commits en anglais.
- Push de la branche.
- PR vers `develop` avec résumé et preuves de validation.
- Après fusion, mettre `develop` local à jour avec `--ff-only`.

## 6. Tenir le suivi projet

Le ticket doit refléter la réalité :

- `Not started` avant prise en charge ;
- `In progress` dès le début du travail ;
- `Done` après fusion et synchronisation de `develop` ;
- lien de PR et note courte sur les décisions ou limites.

Créer un ticket séparé pour une suite utile découverte pendant le travail plutôt
que d'étendre silencieusement le périmètre.
