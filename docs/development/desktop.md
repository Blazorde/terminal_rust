# Développer l'interface desktop

Le prototype desktop enveloppe le moteur Rust existant dans une fenêtre Tauri
2. Le processus Tauri crée une session PTY native, puis démarre le même
exécutable avec l'argument interne `--pty-shell`. Ce mode appelle
`terminal_rust::app::run_interactive()` : saisie, historique, commandes et
prompt ne sont donc pas réimplémentés dans React.

## Prérequis

- Rust stable ;
- Node.js 24 et npm ;
- les dépendances système requises par Tauri 2.

Sous Debian ou Ubuntu, la CI installe notamment WebKitGTK 4.1,
`libayatana-appindicator3-dev`, `librsvg2-dev` et `libxdo-dev`. Windows utilise
WebView2 ; macOS utilise WKWebView. Consulter la documentation Tauri de la
plateforme si le WebView ou les en-têtes natifs manquent.

## Lancer le prototype

```bash
npm ci --prefix desktop
npm run tauri --prefix desktop -- dev
```

La fenêtre doit recevoir le focus terminal après la connexion. Vérifier :

1. saisir `pwd` et lire la sortie réelle ;
2. saisir deux commandes, puis les rappeler avec les flèches haut et bas ;
3. lancer `dashboard`, `clear` et `git status --short --branch` ;
4. utiliser `Ctrl+C` puis continuer la session ;
5. redimensionner la fenêtre et confirmer que la grille PTY suit ;
6. ouvrir le panneau Session avec `Ctrl+,`, le fermer avec `Échap`, puis
   redémarrer la session.

Dans un navigateur classique, Vite rend uniquement un aperçu visuel portant un
message explicite. Aucune sortie de commande n'est simulée.

## Frontière de sécurité

- La WebView appelle seulement `start_session`, `write_session`,
  `resize_session` et `close_session`.
- Le processus Rust possède les handles PTY et les processus enfants.
- La capacité Tauri de la fenêtre reste limitée à `core:default`.
- La politique CSP n'autorise aucune ressource distante en production.
- Les entrées et sorties traversent des tableaux d'octets ; elles ne sont pas
  interpolées dans une commande système.

## Validation

Avant une PR :

```bash
npm ci --prefix desktop
npm run build --prefix desktop
cargo fmt --all -- --check
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
```

La CI répète ces contrôles sous Ubuntu, Windows et macOS. Une exécution locale
sur un seul système prouve le comportement de ce système uniquement ; T121 ne
doit être déclaré totalement validé qu'après une matrice verte ou des preuves
manuelles équivalentes sur les trois plateformes.

## Mesures du spike

Les mesures de démarrage, mémoire et latence doivent être relevées dans une
build de développement puis une build optimisée. Noter le système, le matériel,
la méthode et trois essais au minimum. Ne pas comparer des chiffres provenant
de machines différentes comme s'ils formaient un benchmark unique. Ces mesures
seront consolidées pendant l'audit T124 ; le prototype ne fabrique aucune
donnée de performance dans l'interface.
