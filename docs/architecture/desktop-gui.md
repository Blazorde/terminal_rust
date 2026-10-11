# Architecture cible de l'interface desktop

Ce document décrit les frontières introduites par T118. Tauri 2 est retenu par
l'[ADR-0005](../decisions/0005-desktop-gui-stack.md). Le prototype T121
implémente cette frontière avec `portable-pty` côté Rust et xterm.js côté
présentation. T122 ajoute une persistance versionnée dont la validation et les
accès disque restent dans l'adaptateur Rust.

## Frontières

```text
┌──────────────────────────────────────────────────────────────┐
│ Interface desktop                                            │
│ rendu terminal · réglages · focus · raccourcis · responsive │
└───────────────────────┬──────────────────────────────────────┘
                        │ messages typés / événements
┌───────────────────────▼──────────────────────────────────────┐
│ Adaptateur desktop Rust                                      │
│ sessions PTY · cycle de vie · persistance · capacités OS    │
└───────────────────────┬──────────────────────────────────────┘
                        │ API Rust sans dépendance GUI
┌───────────────────────▼──────────────────────────────────────┐
│ ShellEngine / bibliothèque Rust                              │
│ résultats · commandes · parser · prompt · Git · configuration│
└───────────────┬───────────────────────────────┬──────────────┘
                │                               │
       ┌────────▼────────┐             ┌────────▼────────┐
       │ Binaire CLI     │             │ Adaptateurs OS  │
       │ rustyline       │             │ chemins · PTY   │
       └─────────────────┘             └─────────────────┘
```

## Règles d'architecture

- La bibliothèque Rust ne dépend ni de Tauri, ni de React, ni d'un toolkit GUI.
- Le CLI utilise déjà `ShellEngine`; le desktop consommera le même contrat
  `ExecutionResult` pour les sorties, les erreurs et les changements de dossier.
- La couche desktop ne construit pas des commandes shell par concaténation de
  chaînes. Les entrées utilisateur voyagent comme données vers une API bornée.
- Le processus Rust possède les PTY, les handles de processus, le dossier
  courant et les accès au disque.
- La WebView ne reçoit aucun secret et n'obtient pas d'accès générique au shell
  ou au système de fichiers.
- Les préférences partagées utilisent le schéma versionné existant ou une
  extension rétrocompatible ; une donnée invalide garde un fallback sûr.
- Les polices et assets livrés sont locaux. Une police système monospace reste
  toujours disponible en dernier recours.

## Séquence de livraison

1. **T119** — valider l'ADR et la technologie.
2. **T120** — extraire `terminal-core` sans modifier le comportement CLI.
3. **T121** — ouvrir une fenêtre et connecter une session PTY réelle (prototype
   implémenté ; validation finale dans la matrice des trois OS).
4. **T122** — ajouter réglages, thèmes et persistance (implémenté localement ;
   validation multiplateforme portée par la matrice et T124).
5. **T123** — enrichir les segments de prompt partagés.
6. **T124** — vérifier accessibilité, responsive, performance et trois OS.

Chaque étape possède sa propre branche et sa propre PR. Une fonctionnalité GUI
ne peut pas être déclarée multiplateforme sur la seule preuve d'un poste local ;
la matrice et les captures des trois systèmes sont nécessaires.

## Référence Atelier

L'archive `Terminal de ligne de commande.zip` fournie par le propriétaire est
une référence de composition : fond quadrillé, fenêtre translucide, barre de
titre, zone terminal, pied de contexte et tiroir de réglages. Son code React,
ses commandes simulées et ses données fictives ne sont pas importés. Les écarts
nécessaires seront consignés au fil des tickets GUI.

