# terminal_rust

Mini shell Rust inspire de Bash/Oh My Posh pour Windows et macOS.

## Commandes disponibles

- `pwd`: affiche le dossier courant
- `cd [path]`: change de dossier
- `cd -`: revient au dossier precedent
- `ls`: liste les fichiers et dossiers
- `help`: affiche l'aide
- `exit`: ferme le shell

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

La documentation complete est ici:

[docs/workflows/README.md](docs/workflows/README.md)
