# Moteur de prompt cible

Statut : proposition de conception, non encore implémentée.

## Objectif

Construire un prompt inspiré d'Oh My Posh sans en reproduire toute la complexité
dès la première version. Le système doit composer des segments indépendants et
rester lisible sans couleur, icône ou Git.

## Première version

Segments envisagés :

- dossier courant ;
- branche Git ;
- état propre ou modifié du dépôt ;
- symbole final configurable.

Exemple enrichi :

```text
~/project  main* >
```

Exemple de repli :

```text
C:\project>
```

## Modèle proposé

```rust
struct PromptContext {
    current_dir: PathBuf,
    git: Option<GitContext>,
}

trait Segment {
    fn render(&self, context: &PromptContext) -> Option<RenderedSegment>;
}
```

Le contexte collecte les informations une fois. Les segments décident ensuite
s'ils ont quelque chose à afficher. Le rendu final applique le thème et les
capacités du terminal.

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

1. Enrichir le module Git avec branche et statut.
2. Introduire `PromptContext` sans changer le rendu visible.
3. Extraire un trait ou un enum de segments.
4. Ajouter un thème intégré et un repli sans couleur.
5. Charger une configuration TOML validée.
6. Ajouter cache, mesure de performance et délai des commandes externes.

Chaque étape doit rester fusionnable et testable seule. Les choix définitifs du
modèle de segments et de configuration feront l'objet d'un ADR après prototype.
