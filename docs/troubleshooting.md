# Dépannage

## La commande est introuvable

- Vérifier que le dossier contenant le binaire figure dans le `PATH`.
- Fermer puis rouvrir le terminal après une modification du `PATH`.
- Tester d'abord avec le chemin complet du binaire.
- Sous Windows, conserver l'extension `.exe` lors du diagnostic.

```powershell
& "$env:LOCALAPPDATA\Programs\TerminalRust\terminal_rust.exe" help
```

```bash
"$HOME/.local/bin/terminal_rust" help
```

## Permission refusée sous Linux ou macOS

Le bit exécutable peut être perdu après un téléchargement ou une extraction :

```bash
chmod +x "$HOME/.local/bin/terminal_rust"
```

## macOS refuse d'ouvrir le binaire

L'archive officielle n'est pas encore signée par Apple. Vérifier qu'elle vient
de la page Releases et comparer sa somme SHA-256 avant de l'autoriser dans
**Réglages Système > Confidentialité et sécurité**. Ne désactiver ni Gatekeeper
ni les protections globales du système.

Le message `bad CPU type in executable` indique généralement un Mac Intel :
l'archive publiée actuellement cible Apple Silicon (`arm64`). Compiler depuis
les sources sur la machine en attendant un artefact Intel.

## Windows bloque ou masque le programme

- Extraire le fichier `.zip` avant de lancer le programme.
- Vérifier que le téléchargement vient de la release officielle.
- Comparer la somme avec `SHA256SUMS.txt`.
- Si Windows affiche une protection, examiner l'éditeur et la provenance avant
  de choisir une autorisation ponctuelle ; ne pas désactiver SmartScreen.

## La somme SHA-256 ne correspond pas

Ne pas exécuter le binaire. Supprimer l'archive locale, la télécharger à nouveau
depuis la release officielle et refaire la vérification. Une seconde différence
doit être signalée dans les issues du dépôt.

## Git n'apparaît pas dans le prompt

Le segment Git est optionnel. Vérifier `git --version` et confirmer que le
dossier courant appartient à un dépôt. L'absence de Git n'empêche pas les
commandes internes de fonctionner ; seules les informations Git et la commande
`git` intégrée sont indisponibles.

## L'historique n'est pas conservé

Définir `TERMINAL_RUST_HISTORY_FILE` avec un chemin de fichier dont le dossier
parent existe. L'historique est enregistré lors d'une fermeture normale du
shell.

## Obtenir de l'aide

Noter le système, l'architecture, la version de Terminal Rust, la commande
utilisée et le message complet, puis ouvrir une issue sur
[GitHub](https://github.com/Blazorde/terminal_rust/issues).
