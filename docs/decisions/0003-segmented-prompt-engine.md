# ADR-0003 — Moteur de prompt composé de segments

- Statut : Acceptée
- Date : 2026-10-09
- Décideurs : équipe Terminal Rust

## Contexte

Le prompt doit évoluer d'un chemin suivi de `>` vers une expérience inspirée
d'Oh My Posh : chemin, Git, styles et configuration. Une construction par
concaténations dans `app/prompt.rs` deviendrait difficile à tester et à adapter
aux capacités du terminal.

## Décision

Collecter les données dans un `PromptContext`, puis rendre une liste ordonnée de
segments indépendants. Chaque segment peut s'omettre si ses données ou capacités
manquent. Le thème transforme ensuite les segments en texte final.

La première implémentation reste statique : chemin, contexte Git optionnel et
symbole final sont rendus sans couleur. Le chargement TOML et les thèmes
personnalisés seront ajoutés après stabilisation des interfaces. La collecte Git
est mise en cache pendant 250 ms et un changement de dossier force un nouveau
contexte.

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

## Validation

- Prototype avec chemin, Git et symbole final.
- Tests de rendu avec Git clean, Git dirty et sans Git.
- Repli sans couleur et sans Nerd Font.
- Mesure locale d'environ 50 ms à froid sous Windows et cache testé sans attente
  réelle.
