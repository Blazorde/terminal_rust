# Architecture Decision Records

Un ADR conserve une décision qui influence plusieurs modules, les plateformes
prises en charge ou les futures évolutions. Il explique surtout pourquoi le
choix a été fait et quelles conséquences il faut accepter.

## Registre

| ADR | Statut | Décision |
| --- | --- | --- |
| [0001](0001-command-registry.md) | Acceptée | Registre central des commandes internes |
| [0002](0002-non-blocking-git-detection.md) | Acceptée | Détection Git externe et non bloquante |
| [0003](0003-segmented-prompt-engine.md) | Acceptée | Moteur de prompt composé de segments |
| [0004](0004-allowlisted-git-commands.md) | Acceptée | Raccourcis Git limités à une liste blanche en lecture seule |
| [0005](0005-desktop-gui-stack.md) | Proposée | Couche desktop Tauri autour du moteur Rust |

## Cycle de vie

- `Proposée` : discussion ou prototype en cours.
- `Acceptée` : règle à suivre pour les nouveaux changements.
- `Remplacée` : une décision plus récente prend le relais.
- `Rejetée` : piste documentée mais non retenue.

Copier [le modèle](template.md), choisir le prochain numéro sur quatre chiffres
et ajouter l'ADR à ce registre. Une ADR acceptée n'est pas réécrite pour masquer
le passé : une nouvelle décision la remplace.
