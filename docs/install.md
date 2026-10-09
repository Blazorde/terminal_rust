# Installer Terminal Rust

Les versions prêtes à l'emploi sont publiées sur la
[page Releases](https://github.com/Blazorde/terminal_rust/releases/latest).
Cette installation est manuelle : aucun script ne modifie encore le système.

## Choisir le bon fichier

| Système | Archive |
| --- | --- |
| Windows 64 bits | `terminal_rust-windows-X64.zip` |
| Linux 64 bits | `terminal_rust-linux-X64.tar.gz` |
| macOS Apple Silicon | `terminal_rust-macos-ARM64.tar.gz` |

Les Mac Intel ne disposent pas encore d'une archive officielle. Ils doivent
compiler le projet depuis les sources jusqu'à l'ajout d'un runner x86_64.

## Vérifier le téléchargement

Chaque release fournit `SHA256SUMS.txt`. La somme du fichier téléchargé doit
correspondre à celle indiquée dans ce fichier.

Sous PowerShell :

```powershell
Get-FileHash .\terminal_rust-windows-X64.zip -Algorithm SHA256
```

Sous Linux :

```bash
sha256sum terminal_rust-linux-X64.tar.gz
```

Sous macOS :

```bash
shasum -a 256 terminal_rust-macos-ARM64.tar.gz
```

## Windows

1. Télécharger `terminal_rust-windows-X64.zip`.
2. Extraire l'archive depuis l'Explorateur ou avec PowerShell.
3. Déplacer `terminal_rust.exe` dans un dossier stable, par exemple
   `%LOCALAPPDATA%\Programs\TerminalRust`.
4. Tester le binaire depuis ce dossier.

```powershell
Expand-Archive .\terminal_rust-windows-X64.zip -DestinationPath .\terminal-rust
cd .\terminal-rust
.\terminal_rust.exe help
```

Pour appeler `terminal_rust` depuis tout nouveau terminal, ouvrir **Modifier les
variables d'environnement pour votre compte**, sélectionner `Path`, puis ajouter
le dossier qui contient `terminal_rust.exe`. Fermer et rouvrir le terminal avant
de tester :

```powershell
terminal_rust.exe help
terminal_rust.exe
```

## Linux

1. Télécharger `terminal_rust-linux-X64.tar.gz`.
2. Extraire l'archive et rendre le binaire exécutable.
3. L'installer dans `~/.local/bin`, qui ne demande pas les droits administrateur.

```bash
tar -xzf terminal_rust-linux-X64.tar.gz
chmod +x terminal_rust
mkdir -p "$HOME/.local/bin"
install -m 755 terminal_rust "$HOME/.local/bin/terminal_rust"
```

Si `~/.local/bin` n'est pas déjà dans le `PATH`, ajouter cette ligne à
`~/.profile` pour Bash, puis rouvrir la session :

```bash
export PATH="$HOME/.local/bin:$PATH"
```

Tester ensuite le mode commande puis le shell interactif :

```bash
terminal_rust help
terminal_rust
```

## macOS

L'archive actuelle cible les Mac Apple Silicon. Vérifier l'architecture avec
`uname -m` : la valeur attendue est `arm64`.

```bash
tar -xzf terminal_rust-macos-ARM64.tar.gz
chmod +x terminal_rust
mkdir -p "$HOME/.local/bin"
mv terminal_rust "$HOME/.local/bin/terminal_rust"
```

Avec Zsh, ajouter `~/.local/bin` au `PATH` dans `~/.zshrc`, puis ouvrir un
nouveau terminal :

```bash
export PATH="$HOME/.local/bin:$PATH"
```

Tester ensuite :

```bash
terminal_rust help
terminal_rust
```

Le binaire n'est pas signé par Apple. Si macOS le bloque, vérifier d'abord la
somme SHA-256 et l'origine GitHub, puis autoriser explicitement ce binaire dans
**Réglages Système > Confidentialité et sécurité**.

## Compiler depuis les sources

Cette méthode fonctionne sur Windows, Linux et macOS avec Rust installé :

```bash
git clone https://github.com/Blazorde/terminal_rust.git
cd terminal_rust
cargo build --release --locked
```

Le résultat se trouve dans `target/release/terminal_rust` ou, sous Windows,
`target\release\terminal_rust.exe`.
