# terminal_rust

Mini shell Rust inspire de Bash/Oh My Posh pour Windows et macOS.

## Commandes disponibles terminales classiques:

- `pwd`: affiche le dossier courant
- `cd [path]`: change de dossier
- `cd -`: revient au dossier precedent
- `ls`: liste les fichiers et dossiers
- `mkdir [directory]`: cree un dossier
- `touch [file]`: cree un fichier
- `rm [file/directory]`: supprime un fichier ou dossier
- `cp [source] [destination]`: copie un fichier ou dossier
- `mv [source] [destination]`: deplace ou renomme un fichier ou dossier
- `cat [file]`: affiche le contenu d'un fichier
- `echo [text]`: affiche du texte
- `clear`: nettoie l'ecran
- `history`: affiche l'historique des commandes
- `export [VAR=value]`: definit une variable d'environnement
- `env`: affiche les variables d'environnement
- `alias [name='command']`: cree un alias pour une commande
- `unalias [name]`: supprime un alias
- `help [command]`: affiche l'aide pour une commande specifique
- `help`: affiche l'aide
- `exit`: ferme le shell

## Commandes de gestion de processus:
- `ps`: affiche les processus en cours
- `kill [pid]`: termine un processus par son ID
- `killall [name]`: termine tous les processus portant un nom specifique
- `bg [job]`: met un processus en arriere-plan
- `fg [job]`: ramene un processus en avant-plan
- `jobs`: affiche les processus en arriere-plan
- `wait [pid]`: attend la fin d'un processus
- `nohup [command] &`: lance une commande en arriere-plan sans interruption
- `disown [job]`: detache un processus de la session courante
- `pstree`: affiche les processus sous forme d'arbre
- `top`: affiche les processus en temps reel
- `htop`: affiche les processus en temps reel avec une interface interactive
- `pgrep [pattern]`: recherche les processus correspondant a un motif
- `pkill [pattern]`: termine les processus correspondant a un motif
- `strace [command]`: trace les system calls d'une commande
- `lsof`: affiche les fichiers ouverts par les processus
- `netstat`: affiche les connexions reseau et les ports ouverts
- `ss`: affiche les connexions reseau et les ports ouverts de maniere plus rapide que netstat
- `tcpdump`: capture et analyse le trafic reseau
- `iftop`: affiche l'utilisation du reseau en temps reel
- `iotop`: affiche l'utilisation du disque en temps reel
- `vmstat`: affiche les statistiques du systeme en temps reel
- `free`: affiche l'utilisation de la memoire
- `df`: affiche l'utilisation du disque
- `du`: affiche l'utilisation du disque pour les fichiers et dossiers
- `uptime`: affiche le temps de fonctionnement du systeme
- `dmesg`: affiche les messages du noyau
- `lscpu`: affiche les informations sur le processeur
- `lsblk`: affiche les informations sur les disques et partitions
- `lsusb`: affiche les informations sur les périphériques USB
- `lspci`: affiche les informations sur les périphériques PCI
- `uname -a`: affiche les informations sur le systeme d'exploitation
- `whoami`: affiche le nom de l'utilisateur courant
- `id`: affiche les informations sur l'utilisateur courant
- `groups`: affiche les groupes de l'utilisateur courant
- `chmod [permissions] [file/directory]`: change les permissions d'un fichier ou dossier
- `chown [owner] [file/directory]`: change le propriétaire d'un fichier ou dossier
- `chgrp [group] [file/directory]`: change le groupe d'un fichier ou dossier
- `ln -s [target] [link]`: crée un lien symbolique vers un fichier ou dossier
- `readlink [link]`: affiche la cible d'un lien symbolique
- `stat [file/directory]`: affiche les informations détaillées sur un fichier ou dossier
- `file [file]`: affiche le type d'un fichier
- `basename [path]`: affiche le nom de fichier d'un chemin
- `dirname [path]`: affiche le chemin du dossier d'un chemin
- `which [command]`: affiche le chemin d'une commande
- `whereis [command]`: affiche les chemins d'une commande et de ses fichiers associés
- `locate [pattern]`: recherche des fichiers correspondant à un motif dans une base de données pré-indexée
- `updatedb`: met à jour la base de données utilisée par locate
- `alias [name='command']`: crée un alias pour une commande
- `unalias [name]`: supprime un alias
- `history`: affiche l'historique des commandes
- `!!`: répète la dernière commande
- `!n`: répète la nième commande de l'historique
- `!string`: répète la dernière commande commençant par "string"

## Commande de gestion de paquets:
- `apt [command]`: gestionnaire de paquets pour les distributions basées sur Debian
- `yum [command]`: gestionnaire de paquets pour les distributions basées sur Red
- `dnf [command]`: gestionnaire de paquets pour les distributions basées sur Red Hat
- `pacman [command]`: gestionnaire de paquets pour les distributions basées sur Arch Linux
- `brew [command]`: gestionnaire de paquets pour macOS
- `chocolatey [command]`: gestionnaire de paquets pour Windows
- `cargo [command]`: gestionnaire de paquets pour Rust
- `pip [command]`: gestionnaire de paquets pour Python
- `npm [command]`: gestionnaire de paquets pour Node.js
- `gem [command]`: gestionnaire de paquets pour Ruby
- `composer [command]`: gestionnaire de paquets pour PHP
- `go [command]`: gestionnaire de paquets pour Go
- `dotnet [command]`: gestionnaire de paquets pour .NET
- `nuget [command]`: gestionnaire de paquets pour .NET
- `cargo [command]`: gestionnaire de paquets pour Rust

## Commande git:
- `git init`: initialise un nouveau dépôt Git
- `git clone [repository]`: clone un dépôt Git existant
- `git add [file]`: ajoute un fichier à l'index pour le prochain commit
- `git commit -m "[message]"`: crée un nouveau commit avec les changements ajoutés
- `git push`: pousse les commits locaux vers le dépôt distant
- `git pull`: récupère les changements du dépôt distant et les fusionne avec la branche locale
- `git status`: affiche l'état des fichiers dans le dépôt
- `git log`: affiche l'historique des commits
- `git branch`: affiche les branches du dépôt
- `git checkout [branch]`: change de branche
- `git merge [branch]`: fusionne une branche dans la branche courante
- `git rebase [branch]`: applique les commits de la branche courante sur une autre branche
- `git stash`: sauvegarde les changements non commités pour les appliquer plus tard
- `git stash pop`: applique les changements sauvegardés et les supprime de la liste
- `git stash list`: affiche la liste des changements sauvegardés
- `git stash drop [stash]`: supprime un changement sauvegardé de la liste
- `git remote add [name] [url]`: ajoute un dépôt distant
- `git remote remove [name]`: supprime un dépôt distant
- `git remote -v`: affiche les dépôts distants et leurs URL
- `git tag [name]`: crée une nouvelle étiquette pour un commit
- `git diff`: affiche les différences entre les fichiers modifiés et les fichiers dans l'index
- `git diff --staged`: affiche les différences entre les fichiers ajoutés à l'index et les fichiers dans le dernier commit
- `git reset [file]`: retire un fichier de l'index
- `git reset --hard`: réinitialise la branche courante à l'état du dernier commit, en supprimant tous les changements non commités
- `git revert [commit]`: crée un nouveau commit qui annule les changements d'un commit précédent
- `git cherry-pick [commit]`: applique les changements d'un commit spécifique à la branche courante
- `git bisect`: utilise une recherche binaire pour trouver le commit qui a introduit un bug
- `git blame [file]`: affiche les auteurs des lignes d'un fichier
- `git show [commit]`: affiche les détails d'un commit spécifique
- `git archive [branch] | tar -x`: crée une archive de la branche spécifiée et l'extrait dans le répertoire courant
- `git clean -f`: supprime les fichiers non suivis du répertoire de travail

## Details techniques:
- Pour toutes les commandes, les arguments sont séparés par des espaces. Les arguments entre guillemets sont traités comme une seule unité.
- Le shell supporte les redirections de sortie (`>`, `>>`) et d'entrée (`<`), ainsi que les pipes (`|`) pour chaîner les commandes.
- Les variables d'environnement peuvent être utilisées dans les commandes en les préfixant avec `$`,
- et les alias peuvent être créés pour les commandes courantes pour les rendre plus rapides à taper.
- Le shell supporte également les commandes de gestion de processus pour permettre aux utilisateurs de gérer les processus en cours d'exécution, ainsi que les commandes de gestion de paquets pour faciliter l'installation et la gestion des logiciels.
- Enfin, le shell intègre des commandes git pour permettre aux utilisateurs de gérer leurs dépôts git directement depuis le terminal.

## Developpement

Commandes utiles:

```bash
cargo fmt
cargo check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

## Branches et workflows

Le projet utilise une politique stricte:

- `main` accepte uniquement les PR venant de `develop`
- `develop` accepte uniquement les PR venant de branches:
  - `feat/*`
  - `fix/*`
  - `chore/*`
  - `refractor/*`
  - `hotfix/*`
- les checks requis sont `CI / Required CI gate` et `PR policy / Required PR policy gate`
- les comptes autorises a bypass sont listes dans `.github/bypass-owners.txt`

La documentation complete est ici:

[docs/workflows/README.md](docs/workflows/README.md)
