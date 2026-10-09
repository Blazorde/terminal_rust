# Checklist qualité MVP

Cette checklist complète les tests automatisés avant une livraison ou une PR de
stabilisation. Elle doit être exécutée depuis la racine du dépôt, sur une branche
à jour de `develop`.

## Vérifications automatisées

```bash
cargo fmt --all -- --check
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
```

La CI doit ensuite réussir sur Windows, Linux et macOS. Les tests CLI couvrent
les chemins contenant des espaces et tolèrent les fins de ligne propres à chaque
plateforme.

## Parcours manuel principal

Démarrer le shell avec `cargo run`, puis vérifier :

1. `help` affiche toutes les commandes publiques, y compris `help`.
2. `pwd`, `mkdir quality-check`, `touch "quality-check/note.txt"`,
   `ls -la quality-check` et `cat "quality-check/note.txt"` fonctionnent.
3. `cd quality-check`, `cd ..` et `cd -` changent correctement de dossier.
4. `git status --short --branch`, `git branch` et `git log -n 3` restent limités
   aux sous-commandes Git autorisées.
5. `dashboard` affiche les sections système, performance, stockage et réseau,
   puis contexte, avec les mesures processeur et mémoire, sans exiger Fastfetch
   ni une Nerd Font.
6. `clear` efface un terminal compatible et ne ferme pas le shell.
7. La flèche haute rappelle une commande de l'historique.
8. `rm quality-check/note.txt`, puis `rm -d quality-check`, nettoient les
   fichiers créés pour la vérification.
9. Une commande inconnue affiche une erreur utile et le shell reste ouvert.
10. `exit` ferme normalement le shell et enregistre l'historique si sa
    persistance est disponible.

## Replis à contrôler

- Hors dépôt Git, le prompt reste utilisable sans segment Git.
- Sans Git installé, seules les capacités Git sont indisponibles.
- Avec une sortie redirigée ou `TERM=dumb`, le dashboard utilise du texte ASCII
  sans séquences ANSI.
- Sans dossier personnel détectable, le shell continue sans persistance de
  l'historique.

Supprimer les données temporaires après la passe. Tout défaut qui demande une
évolution importante doit être isolé dans un ticket au lieu d'élargir la PR de
stabilisation.
