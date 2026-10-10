# ADR-0005 — Couche desktop Tauri autour du moteur Rust

- Statut : Proposée
- Date : 2026-10-10
- Décideurs : équipe Terminal Rust

## Contexte

T118 demande une interface desktop fidèle à la référence Atelier sur Windows,
macOS et Linux, sans remplacer le moteur du shell ni le mode CLI. La couche GUI
doit rendre un terminal réel, un tiroir de réglages dense et responsive, six
thèmes et une navigation clavier complète. Elle ne doit pas exécuter les
commandes dans JavaScript ni dépendre d'une police ou d'une ressource distante.

La référence `Terminal de ligne de commande.zip` est un prototype React qui
simule les commandes. Elle sert uniquement de preuve visuelle et fonctionnelle :
son moteur, ses données fictives et ses instructions embarquées ne sont pas une
base d'exécution.

## Décision proposée

Utiliser **Tauri 2** pour la fenêtre desktop et une interface locale
TypeScript/React pour le rendu, sous réserve de validation par le propriétaire
du projet. Le moteur, les commandes, Git, les chemins, la configuration et les
sessions terminal restent en Rust.

La frontière est stricte :

1. `terminal-core` expose les commandes, le contexte, le prompt et les erreurs
   sans dépendre d'une interface.
2. Le binaire CLI actuel devient un consommateur de `terminal-core` et conserve
   son comportement public.
3. Le processus Rust Tauri possède les sessions PTY, le système de fichiers et
   la configuration. Il émet des événements de sortie et accepte des requêtes
   typées et minimales.
4. La WebView affiche la grille terminal et les réglages. Elle ne lance jamais
   directement un processus, Git ou une commande système.
5. Les capacités Tauri sont limitées à la fenêtre principale et aux commandes
   explicitement exposées. Aucun contenu distant n'est chargé.

Le premier prototype doit utiliser une API PTY multiplateforme derrière un
trait interne. [`portable-pty`](https://docs.rs/portable-pty/latest/portable_pty/)
est le candidat initial, car il sélectionne une implémentation native sur Unix
et Windows. Le choix du parseur de flux terminal reste isolé ;
[`alacritty_terminal`](https://docs.rs/alacritty_terminal/latest/alacritty_terminal/)
est à évaluer dans un spike avant engagement.

## Options considérées

| Critère | Tauri 2 | `iced` | `egui` / `eframe` |
| --- | --- | --- | --- |
| Fidélité à la maquette Atelier | Élevée : CSS, grille, typographie et responsive proches de la référence | Moyenne : composants et mise en page à reconstruire | Moyenne à faible : excellent pour les outils, moins adapté à cette composition précise |
| Partage du moteur Rust | Fort via commandes et événements typés | Direct, tout Rust | Direct, tout Rust |
| Terminal complexe | Rendu DOM/canvas possible, PTY conservé en Rust | Widget terminal personnalisé à construire | Widget terminal personnalisé à construire |
| Accessibilité | Sémantique Web disponible, à auditer et tester | Sémantique dépendante des widgets et de la plateforme | Intégration native à vérifier pour chaque contrôle personnalisé |
| Responsive et réglages | Naturels avec CSS et composants structurés | Faisables avec davantage de code de layout | Faisables, mais l'immediate mode augmente le travail d'état et de composition |
| Dépendances système | WebView système ; WebKitGTK à prévoir sur Linux | Renderer natif/GPU | Renderer natif/GPU |
| Risque principal | Sécuriser l'IPC et maintenir deux langages | Écosystème déclaré expérimental et documentation exigeante | Versions fréquemment cassantes et fidélité visuelle plus coûteuse |

Les sources officielles décrivent Tauri comme une architecture Rust + WebView
reposant sur TAO/WRY, avec des capacités permettant de réduire l'exposition de
l'interface au système :
[architecture](https://tauri.app/concept/architecture/) et
[capabilities](https://v2.tauri.app/security/capabilities/). `iced` se présente
comme une bibliothèque multiplateforme typée inspirée d'Elm, encore
expérimentale ([documentation](https://docs.iced.rs/iced/index.html)). `egui` est
une GUI immediate mode en Rust, disponible nativement via `eframe`, mais sa
documentation avertit de changements cassants fréquents
([documentation](https://docs.rs/egui/latest/egui/)).

## Conséquences

### Positives

- La référence visuelle peut être reproduite sans porter toute sa composition
  vers un toolkit de widgets différent.
- Le CLI et la GUI partagent le même moteur Rust et les mêmes tests métier.
- Les assets, polices de secours et thèmes peuvent fonctionner entièrement hors
  ligne.
- L'interface Web reste confinée à la présentation ; les capacités sensibles
  restent dans le processus Rust.

### Négatives

- Le projet devient polyglotte pour la couche GUI et ajoute une toolchain
  frontend de build.
- Linux nécessite les bibliothèques WebKitGTK attendues par Tauri.
- La sécurité de l'IPC, la politique de contenu et la validation des messages
  deviennent des responsabilités explicites.
- Le rendu peut différer légèrement entre les WebViews système ; les trois OS
  doivent être capturés et testés.

## Alternatives rejetées à ce stade

- **Intégrer le prototype React tel quel** : refusé, car ses commandes et ses
  dépôts sont simulés et son chargement de polices distantes viole les fallbacks
  demandés.
- **Réécrire le moteur en TypeScript** : refusé ; Rust reste la source de vérité.
- **Commencer directement avec `iced` ou `egui`** : non retenu pour le premier
  prototype, car le coût de reconstruction du terminal et des réglages retarde
  la validation du produit. Ces options restent viables si une interface 100 %
  Rust est une contrainte prioritaire confirmée.

## Validation

L'ADR devient `Acceptée` seulement après confirmation du choix Tauri. Le spike
T121 devra ensuite prouver :

- lancement de la même session réelle sur Windows, macOS et Linux ;
- aucune exécution de commande dans la WebView ;
- mode CLI inchangé et suite de tests existante verte ;
- navigation clavier, focus visible, zoom et petite fenêtre ;
- démarrage sans réseau, Git ni Nerd Font ;
- capacités Tauri minimales et politique de contenu sans source distante ;
- mesures documentées du démarrage, de la mémoire et de la latence d'entrée.

