# Moteur de prompt

Statut : première version implémentée, configuration et thèmes à venir.

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
~/project  main* >
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
```

`Prompt` vit pendant toute la session interactive. Le module `git` collecte la
branche et l'état avec une seule commande porcelain v2 ; le module `cache`
conserve ce résultat pendant 250 ms, y compris l'absence de contexte Git. Le
rendu n'affiche le segment que lorsque ces données sont disponibles.

La commande mesurée sur le dépôt du projet sous Windows prend environ 50 ms à
froid. Le cache évite de répéter ce coût lors de rendus rapprochés tout en
bornant l'obsolescence à 250 ms. Les tests injectent le temps et le fournisseur
de données afin de vérifier le TTL sans attente réelle.

## Configuration future

Une configuration TOML pourra définir l'ordre et le style sans permettre
l'exécution arbitraire de commandes :

```toml
[[segments]]
type = "path"

[[segments]]
type = "git"
show_status = true

[prompt]
symbol = ">"
icons = "auto"
```

## Étapes recommandées

1. Extraire un trait ou un enum de segments à partir du prototype actuel.
2. Ajouter un thème intégré et un repli sans couleur.
3. Charger une configuration TOML validée.
4. Ajouter une limite de durée aux commandes externes.

Chaque étape doit rester fusionnable et testable seule. L'ADR-0003 fixe la
direction par segments ; les interfaces pourront évoluer avec la configuration.
