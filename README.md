# Terminal Rust

Terminal Rust est un mini shell écrit en Rust. Le projet sert à apprendre comment construire un terminal proprement: parsing des commandes, autocomplétion, gestion d'erreurs, commandes internes, tests et workflows GitHub.

Le but n'est pas de remplacer Bash, PowerShell ou Zsh dès maintenant. Le but est de construire progressivement un shell fiable, lisible, maintenable et proche des bonnes pratiques Rust.

## Sommaire

- [Fonctionnalités](#fonctionnalités)
- [Installation](#installation)
- [Lancement](#lancement)
- [Commandes disponibles](#commandes-disponibles)
- [Historique interactif](#historique-interactif)
- [Détection Git](#détection-git)
- [Commandes Git développeur](#commandes-git-développeur)
- [Dashboard système](#dashboard-système)
- [Preset Fastfetch](#preset-fastfetch)
- [Options de `ls`](#options-de-ls)
- [Options de `rm`](#options-de-rm)
- [Architecture](#architecture)
- [Documentation technique](#documentation-technique)
- [Gestion des erreurs](#gestion-des-erreurs)
- [Développement](#développement)
- [Tests et qualité](#tests-et-qualité)
- [Workflows GitHub](#workflows-github)
- [Roadmap](#roadmap)
- [Licence](#licence)

## Fonctionnalités

- Shell interactif avec historique et autocomplétion grâce à `rustyline`.
- Commandes internes séparées par modules.
- Gestion d'erreurs dédiée par commande.
- Parsing des arguments avec support des guillemets.
- Commande `ls` avec plusieurs options proches de Linux.
- Commande `rm` avec options et confirmation sur les suppressions dangereuses.
- Détection silencieuse du contexte Git, prête pour les futurs segments du prompt.
- Dashboard système intégré avec CPU, mémoire, stockage, réseau et contexte Git.
- Preset Fastfetch portable et personnalisable.
- Tests unitaires et tests d'intégration qui lancent le vrai binaire.
- GitHub Actions sous Ubuntu, Windows et macOS pour vérifier le formatage, la
  compilation, Clippy et les tests.

## Installation

Prérequis:

- Rust installé avec `rustup`
- Cargo disponible dans le terminal

Vérifier l'installation:

```bash
rustc --version
cargo --version
```

Cloner le projet:

```bash
git clone https://github.com/Blazorde/terminal_rust.git
cd terminal_rust
```

Installer ou compiler:

```bash
cargo build
```

## Lancement

Lancer le shell en mode développement:

```bash
cargo run
```

Compiler puis lancer le binaire:

```bash
cargo build --release
./target/release/terminal_rust
```

Sur Windows, le binaire release se trouve ici:

```powershell
.\target\release\terminal_rust.exe
```

## Commandes disponibles

| Commande | Description |
| --- | --- |
| `pwd` | Affiche le dossier courant |
| `cd [path]` | Change de dossier |
| `cd -` | Revient au dossier précédent |
| `ls [options] [path...]` | Liste les fichiers et dossiers |
| `mkdir [directory]` | Crée un dossier |
| `touch [file]` | Crée un fichier vide ou garde le fichier existant |
| `cat [file]` | Affiche le contenu d'un fichier |
| `rm [options] [path...]` | Supprime des fichiers ou dossiers selon les options |
| `clear` | Efface l'écran d'un terminal compatible ANSI |
| `dashboard` | Affiche un tableau de bord système et performance |
| `git <status\|branch\|log> [options]` | Lance un raccourci Git autorisé dans le dossier courant |
| `help` | Affiche l'aide |
| `exit` | Ferme le shell |
| `quit` | Ferme le shell |

Exemples:

```bash
pwd
cd src
cd -
ls -la
mkdir test
touch test/file.txt
cat test/file.txt
rm -i test/file.txt
rm -r test
clear
dashboard
git status --short --branch
git branch --all
git log --oneline -10
exit
```

## Historique interactif

En mode interactif, les commandes non vides sont ajoutées à l'historique de
`rustyline` et restent accessibles avec les flèches haut et bas pendant toute la
session.

Par défaut, cet historique reste uniquement en mémoire. Pour le conserver entre
deux lancements, définir la variable `TERMINAL_RUST_HISTORY_FILE` avec le chemin
du fichier à utiliser. Le dossier parent doit déjà exister.

Sous PowerShell, par exemple:

```powershell
$env:TERMINAL_RUST_HISTORY_FILE = "$HOME\.terminal_rust_history"
cargo run
```

Si le fichier n'existe pas encore, il sera créé à la fermeture normale du shell.
Une erreur de lecture ou d'écriture est affichée sans empêcher l'utilisation de
l'historique en mémoire.

## Détection Git

Le module `src/git/` fournit la première brique du futur prompt enrichi. Il
détermine si un dossier appartient à un dépôt Git avec :

```text
git -C <directory> rev-parse --is-inside-work-tree
```

Cette détection est volontairement non bloquante : un dossier hors dépôt, un
chemin invalide ou l'absence de Git retournent simplement `false`. Aucun message
d'erreur Git n'est affiché et le shell reste utilisable sur macOS, Linux et
Windows.

## Commandes Git développeur

Le shell expose un sous-ensemble volontairement limité de Git : `status`,
`branch` et `log`. Les options de consultation sont transmises à Git, tandis
que les commandes et arguments qui modifient un dépôt, comme `push`, `reset`,
`clean`, `branch -d` ou la création d'une branche, sont refusés.

```bash
git status --short --branch
git branch --all
git log --oneline --decorate -10
```

La sortie standard, la sortie d'erreur et le code de sortie de Git sont
conservés. Si Git n'est pas installé, le mini-shell affiche une erreur claire
sans s'arrêter. Le lancement utilise `git -C <dossier>` et reste identique sur
Windows, macOS et Linux.

## Dashboard système

La commande `dashboard` affiche directement dans le shell :

- système, noyau, machine et uptime ;
- modèle, charge et nombre de cœurs CPU ;
- mémoire, swap et nombre de processus ;
- utilisation du disque courant ;
- volumes réseau reçus et envoyés ;
- dossier courant et présence d'un dépôt Git.

```bash
cargo run -- dashboard
```

Le rendu utilise des couleurs et des bordures Unicode dans un terminal
interactif. Si la sortie est redirigée, si `TERM=dumb` ou si `NO_COLOR` est
défini, il passe automatiquement en ASCII sans couleur.

## Preset Fastfetch

Une configuration Fastfetch complémentaire est fournie dans
[`config/fastfetch/config.jsonc`](config/fastfetch/config.jsonc). Elle fonctionne
sans Nerd Font et regroupe les informations système importantes avec un thème
cyan, vert, jaune et magenta.

```bash
fastfetch --config config/fastfetch/config.jsonc
```

Fastfetch reste optionnel : il n'est ni installé ni requis par Terminal Rust.
Le guide de personnalisation se trouve dans
[`config/fastfetch/README.md`](config/fastfetch/README.md).

## Options de `ls`

La commande `ls` accepte plusieurs options courtes combinables:

| Option | Description |
| --- | --- |
| `-1` | Affiche un élément par ligne |
| `-a` | Affiche aussi les fichiers cachés |
| `-A` | Affiche presque tous les fichiers cachés |
| `-d` | Affiche le dossier lui-même au lieu de son contenu |
| `-F` | Ajoute un indicateur selon le type de fichier |
| `-h` | Affiche les tailles sous forme lisible |
| `-l` | Affichage long |
| `-p` | Ajoute `/` après les dossiers |
| `-R` | Parcours récursif |
| `-r` | Inverse l'ordre de tri |
| `-S` | Trie par taille |
| `-t` | Trie par date de modification |

Exemples:

```bash
ls
ls -la
ls -lh src
ls -R .
ls -S
ls -t
```

## Options de `rm`

La commande `rm` est volontairement prudente. Si elle détecte une suppression récursive dangereuse, elle demande une confirmation.

| Option | Version longue | Description |
| --- | --- | --- |
| `-d` | `--dir` | Supprime un dossier vide |
| `-f` | `--force` | Ignore les chemins inexistants |
| `-i` | `--interactive` | Demande confirmation avant suppression |
| `-r` | `--recursive` | Supprime récursivement un dossier |
| `-R` | `--recursive` | Alias de `-r` |
| `-v` | `--verbose` | Affiche les chemins supprimés |
| `--` | | Arrête le parsing des options |

Exemples:

```bash
rm file.txt
rm file1.txt file2.txt
rm -i file.txt
rm -f missing.txt
rm -d empty_folder
rm -r folder
rm -rf folder
rm -- -file-starting-with-dash
```

Comportement de sécurité:

- `rm folder` refuse de supprimer un dossier sans option.
- `rm -d folder` supprime uniquement un dossier vide.
- `rm -r folder` demande confirmation avant suppression récursive.
- `rm -rf folder` demande aussi confirmation dans ce projet, même si Linux classique ne le fait pas toujours.

## Architecture

Le code est organisé pour éviter les gros fichiers difficiles à maintenir.

```text
src/
  main.rs
  app/
    command.rs
    completion.rs
    help.rs
    parser.rs
    prompt.rs
    runner.rs
  commands/
    cd.rs
    clear.rs
    dashboard.rs
    mkdir.rs
    pwd.rs
    rm.rs
    touch.rs
    ls/
      display.rs
      fs.rs
      item.rs
      options.rs
      sort.rs
  git/
    mod.rs
  errors/
    cd.rs
    command.rs
    ls.rs
    mkdir.rs
    rm.rs
    touch.rs
```

Rôle des dossiers principaux:

| Dossier | Rôle |
| --- | --- |
| `app/` | Boucle du shell, parsing, autocomplétion, dispatch des commandes |
| `commands/` | Logique métier des commandes |
| `commands/ls/` | Sous-modules dédiés à `ls`, car la commande est plus complexe |
| `errors/` | Types d'erreurs séparés par commande |

Le fichier `src/app/command.rs` utilise une table de commandes. Cela évite un énorme `match` qui grossit à chaque nouvelle commande.

## Documentation technique

La documentation détaillée est indexée dans [`docs/README.md`](docs/README.md).
Elle couvre notamment :

- l'architecture actuelle et l'architecture cible ;
- la compatibilité Windows, macOS et Linux ;
- le futur moteur de prompt à segments inspiré d'Oh My Posh ;
- les décisions d'architecture (ADR) ;
- les règles de contribution, de tests et de traitement des tickets.

Les règles destinées aux contributeurs automatisés se trouvent dans
[`AGENTS.md`](AGENTS.md).

## Gestion des erreurs

Chaque commande importante possède son type d'erreur dédié:

```text
CdError
LsError
MkdirError
RmError
TouchError
```

Cette approche permet:

- des messages d'erreur clairs;
- des tests précis;
- une séparation propre entre la logique métier et l'affichage;
- une meilleure maintenance quand les commandes deviennent plus complètes.

Exemple simplifié:

```rust
pub enum RmError {
    MissingTarget,
    NotFound(PathBuf),
    IsDirectory(PathBuf),
    DeleteFailed { path: PathBuf, source: io::Error },
}
```

## Développement

Commandes utiles pendant le développement:

```bash
cargo fmt
cargo check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

Ajouter une nouvelle commande:

1. Créer `src/commands/ma_commande.rs`.
2. Créer `src/errors/ma_commande.rs` si la commande peut échouer.
3. Exporter les modules dans `src/commands/mod.rs` et `src/errors/mod.rs`.
4. Ajouter la commande dans la table de `src/app/command.rs`.
5. Ajouter l'aide dans `src/app/help.rs`.
6. Ajouter l'autocomplétion dans `src/app/completion.rs`.
7. Ajouter des tests sur les comportements importants.

## Tests et qualité

Avant d'ouvrir une pull request:

```bash
cargo fmt --all -- --check
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
```

Les tests couvrent notamment:

- le parsing des arguments;
- l'autocomplétion des commandes;
- `cd`;
- `ls`;
- `mkdir`;
- `touch`;
- `rm` ;
- l'exécution CLI réelle de `pwd` et `ls` ;
- les sorties d'erreur et codes de retour des commandes invalides.

## Workflows GitHub

Le projet utilise une politique stricte de branches et de pull requests.

Branches principales:

- `main` accepte uniquement les pull requests venant de `develop`.
- `develop` accepte uniquement les pull requests venant de branches nommées:
  - `feat/*`
  - `fix/*`
  - `chore/*`
  - `refractor/*`
  - `hotfix/*`

Checks attendus:

- `CI / Required CI gate`
- `PR policy / Required PR policy gate`

Les comptes autorisés à bypass sont listés ici:

```text
.github/bypass-owners.txt
```

Documentation complète:

[Lire la documentation des workflows](docs/workflows/README.md)

## Roadmap

Idées de commandes ou fonctionnalités à ajouter plus tard:

- `cp`
- `mv`
- `echo`
- `history`
- variables d'environnement
- alias
- redirections
- pipes
- meilleure gestion des permissions
- interface terminal graphique dédiée

## Licence

Ce projet est sous licence MIT.
