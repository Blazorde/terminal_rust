# Prototype TUI à panneaux

Ce document fixe la direction technique du ticket T111. Le prototype reste une
expérience séparée : il ne remplace ni le shell CLI, ni l'interface desktop
Tauri, et ne lance aucune commande.

## Choix technique

Le prototype utilise **Ratatui 0.30** avec le backend **Crossterm 0.29**. Ce
couple fournit les primitives de layout, le rendu différentiel, les événements
clavier et le redimensionnement sur Windows, macOS et Linux. Les dépendances
sont optionnelles derrière la feature `tui-prototype` afin que le binaire CLI
normal ne les compile pas.

La solution est volontairement un exemple Cargo plutôt qu'un nouveau mode du
shell :

```console
cargo run --example tui-layout --features tui-prototype
```

Ratatui initialise le mode brut et l'écran alternatif, puis restaure le terminal
à la sortie et lors d'un panic. Crossterm ne livre que les événements ; le
prototype ignore les événements non clavier et laisse le prochain rendu
recalculer les rectangles après un événement de redimensionnement.

## Composition

Le prototype affiche deux zones avec une barre d'aide :

- `Terminal`, zone de sortie factice qui représente le futur terminal actif ;
- `Project & logs`, zone factice pour fichiers, Git et journaux ;
- un split à 65/35, horizontal ou vertical.

Les deux zones sont rendues depuis un état minimal et testable. Aucun accès au
système de fichiers, à Git, au réseau ou au moteur du shell n'est ajouté.

## Raccourcis

| Entrée | Action |
| --- | --- |
| `Tab`, `Shift+Tab` | déplacer le focus vers l'autre panneau |
| flèches | déplacer le focus vers l'autre panneau |
| `s` | basculer le split horizontal/vertical |
| `q`, `Esc`, `Ctrl+C` | quitter et restaurer le terminal |

La bordure cyan constitue l'indicateur de focus. Crossterm émet aussi des
événements de relâchement sous Windows ; seuls les événements `Press` sont
traités pour éviter les doubles actions.

## Frontière avec le produit

Une intégration future devra remplacer les données factices par des vues
typées, sans déplacer l'exécution de commandes hors de `ShellEngine`. Le
terminal desktop continuera d'utiliser ses sessions PTY isolées. Avant toute
promotion de ce prototype, il faudra prouver le rendu, les raccourcis et la
restauration du terminal sur les trois systèmes ciblés.

Sources : [installation Ratatui](https://ratatui.rs/installation/),
[backends Ratatui](https://ratatui.rs/concepts/backends/) et
[Crossterm](https://docs.rs/crate/crossterm/latest).
