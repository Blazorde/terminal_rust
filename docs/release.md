# Builds de distribution

Le workflow `.github/workflows/release.yml` produit des archives contenant le
binaire optimisé de Terminal Rust pour Windows, Linux et macOS.

## Déclenchement

Le workflow peut être lancé manuellement depuis l'onglet **Actions** de GitHub.
Il est aussi exécuté lorsqu'un tag respectant la forme `v*.*.*`, par exemple
`v0.1.0`, est poussé.

Le workflow ne crée pas encore de GitHub Release. Cette publication est une
étape séparée afin de pouvoir contrôler la version et les notes de version.

## Contenu des artefacts

Chaque système compile avec :

```bash
cargo build --release --locked
```

Le binaire est testé avec la commande `help` avant sa mise en archive. Les noms
d'artefacts incluent le système et l'architecture fournie par le runner :

```text
terminal_rust-linux-<architecture>.tar.gz
terminal_rust-macos-<architecture>.tar.gz
terminal_rust-windows-<architecture>.zip
```

Les artefacts sont conservés pendant 14 jours dans l'exécution GitHub Actions.

## Télécharger un build

1. Ouvrir l'onglet **Actions** du dépôt.
2. Choisir le workflow **Release builds**.
3. Ouvrir une exécution terminée avec succès.
4. Télécharger l'artefact correspondant à son système.
5. Extraire l'archive avant de lancer le binaire.

Sous Linux et macOS, si le système a perdu le bit exécutable pendant le
téléchargement ou l'extraction :

```bash
chmod +x terminal_rust
./terminal_rust help
```

Sous Windows :

```powershell
.\terminal_rust.exe help
```

## Validation locale

Un poste local ne valide que son propre système. Sous Windows, par exemple :

```powershell
cargo build --release --locked
.\target\release\terminal_rust.exe help
```

La preuve multiplateforme est fournie par les trois jobs du workflow GitHub
Actions. Un échec de compilation, de smoke test, d'archivage ou d'upload fait
échouer le job correspondant.
