# Builds de distribution

Le workflow `.github/workflows/release.yml` produit des archives contenant le
binaire optimisé de Terminal Rust pour Windows, Linux et macOS.

## Déclenchement

Le workflow peut être lancé manuellement depuis l'onglet **Actions** de GitHub.
Il est aussi exécuté lorsqu'un tag respectant la forme `v*.*.*`, par exemple
`v0.1.0`, est poussé.

Pour un lancement manuel, les archives restent disponibles comme artefacts du
workflow. Pour un tag de version, le workflow publie aussi une GitHub Release,
ses trois archives, leurs sommes SHA-256 et les notes extraites de
`CHANGELOG.md`.

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

## Télécharger une version stable

La [page Releases](https://github.com/Blazorde/terminal_rust/releases/latest)
fournit les archives Windows, Linux et macOS sans nécessiter de compte GitHub.
Le fichier `SHA256SUMS.txt` permet de vérifier leur intégrité.

## Télécharger un build temporaire

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

## Publier une version

1. Mettre à jour la version dans `Cargo.toml` et `Cargo.lock`.
2. Ajouter une section `## [x.y.z]` dans `CHANGELOG.md`.
3. Fusionner la version validée dans `main`.
4. Créer et pousser un tag minuscule `vx.y.z` sur ce commit.
5. Vérifier que les trois builds et le job **Publish GitHub Release** réussissent.

Le job de publication refuse un tag sans section correspondante dans le
changelog et vérifie que le tag existe déjà sur GitHub.
