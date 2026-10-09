# ADR-0003 — Moteur de prompt composé de segments

- Statut : Proposée
- Date : 2026-10-09
- Décideurs : équipe Terminal Rust

## Contexte

Le prompt doit évoluer d'un chemin suivi de `>` vers une expérience inspirée
d'Oh My Posh : chemin, Git, styles et configuration. Une construction par
concaténations dans `app/prompt.rs` deviendrait difficile à tester et à adapter
aux capacités du terminal.

## Décision proposée

Collecter les données dans un `PromptContext`, puis rendre une liste ordonnée de
segments indépendants. Chaque segment peut s'omettre si ses données ou capacités
manquent. Le thème transforme ensuite les segments en texte final.

La première implémentation restera statique. Le chargement TOML et les thèmes
personnalisés seront ajoutés après stabilisation des interfaces.

## Options considérées

- Chaîne construite directement : faible coût initial, forte dette rapide.
- Templates avec expressions : flexibles mais trop complexes et risqués pour le
  MVP.
- Segments typés : extensibles, testables et compatibles avec une configuration
  déclarative future.

## Conséquences

### Positives

- Tests unitaires sans terminal réel.
- Repli facile lorsqu'un segment est indisponible.
- Ordre et thème configurables à terme.

### Négatives

- Plus de types avant que le prompt visuel soit riche.
- Nécessité de définir largeur, couleurs et cache avec soin.

## Validation avant acceptation

- Prototype avec segments `path`, `git` et `symbol`.
- Rendu correct avec et sans Git.
- Mode sans couleur et sans Nerd Font.
- Mesure du coût du prompt dans un dépôt de taille réaliste.
