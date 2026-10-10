# Moteur de prompt

Statut : modèle et renderer ANSI implémentés, contrat de configuration défini,
chargement et thèmes dynamiques à venir.

## Objectif

Construire un prompt inspiré d'Oh My Posh sans en reproduire toute la complexité
dès la première version. Le système doit composer des segments indépendants et
rester lisible sans couleur, icône ou Git.

## Première version actuelle

Le rendu actuel compose :

- dossier courant ;
- branche Git ;
- état propre ou modifié du dépôt ;
- symbole final statique.

Exemple enrichi :

```text
~/project  git:main* >
```

Exemple de repli :

```text
C:\project>
```

## Modèle actuel

```rust
struct Prompt {
    git_cache: GitPromptCache,
}

struct PromptGitInfo {
    branch: String,
    dirty: bool,
}

struct PromptSegment {
    kind: SegmentKind,
    label: Option<String>,
    value: Option<String>,
    style_key: String,
    icon: Option<SegmentIcon>,
}
```

`Prompt` vit pendant toute la session interactive. Le module `git` collecte la
branche et l'état avec une seule commande porcelain v2 ; le module `cache`
conserve ce résultat pendant 250 ms, y compris l'absence de contexte Git. Le
rendu n'affiche le segment que lorsque ces données sont disponibles.

`PromptSegment` est le contrat commun entre collecte et rendu. Il transporte une
valeur et des références sémantiques de style ou d'icône, jamais un code ANSI.
Une valeur absente ou vide rend le segment invisible. Le modèle couvre le
chemin, Git et le symbole dans la version actuelle ; sa structure commune pourra
être réutilisée pour l'utilisateur, l'heure ou le statut de sortie.

Le module `render::palette` centralise les rôles sémantiques `primary`,
`secondary`, `success`, `warning`, `error` et `muted`. Les segments ne connaissent
que ces noms : la palette injectée décide de leur couleur sans modifier la logique
du chemin ou de Git. Le module `render::ansi` traduit ensuite la palette en
séquences ANSI portables. Chaque segment stylé se termine par un reset, afin qu'une couleur ne
déborde jamais sur la saisie ou la sortie suivante. Le renderer propose les huit
couleurs ANSI de base, le foreground, le background, le gras et le mode atténué.
Il produit automatiquement du texte brut lorsque la sortie standard n'est pas
un terminal, lorsque `NO_COLOR` est défini ou lorsque `TERM=dumb`. Sous Windows,
la couleur n'est activée que lorsqu'un terminal ANSI connu est détecté (Windows
Terminal, VS Code, ConEmu, ANSICON ou un environnement qui expose `TERM`) ; la
console historique conserve ainsi un prompt lisible.

Le module `render::icons` contient tous les glyphes Nerd Font. Le renderer choisit
entre le glyphe et le fallback ASCII porté par `SegmentIcon`. Les modes `always`,
`never` et `auto` sont indépendants de la couleur ; le mode automatique reste
prudent et exige une confirmation explicite de la police configurée.

Le module `render::separators` applique la même détection aux séparateurs
Powerline. Il colore le glyphe avec le rôle du segment précédent et retourne un
fallback ASCII lorsque la capacité Nerd Font n'est pas confirmée. La collecte
des données et les segments restent ainsi indépendants de la décoration.

La commande mesurée sur le dépôt du projet sous Windows prend environ 50 ms à
froid. Le cache évite de répéter ce coût lors de rendus rapprochés tout en
bornant l'obsolescence à 250 ms. Les tests injectent le temps et le fournisseur
de données afin de vérifier le TTL sans attente réelle.

## Contrat de configuration

La [spécification de personnalisation](../customization.md) sépare désormais la
sélection des segments de leur apparence. Le fichier utilisateur choisira le
thème, l'ordre et les options fonctionnelles ; le thème décrira uniquement le
rendu. Aucun des deux formats ne permet l'exécution arbitraire de commandes.

Exemple :

```toml
version = 1
theme = "default"

[prompt]
icons = "auto"
segments = ["path", "git", "symbol"]
```

Les exemples canoniques sont `config/default.toml` et
`config/themes/default.toml`. Le chargement est volontairement reporté aux
tickets d'implémentation suivants.

## Étapes recommandées

1. Charger et valider la configuration TOML versionnée.
2. Connecter les styles du thème au renderer ANSI.
3. Ajouter une limite de durée aux commandes externes.

Chaque étape doit rester fusionnable et testable seule. L'ADR-0003 fixe la
direction par segments ; les interfaces pourront évoluer avec la configuration.
