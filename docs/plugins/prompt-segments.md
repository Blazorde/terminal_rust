# Segments de prompt des plugins

Un plugin peut déclarer un futur segment de prompt dans son manifeste. Cette
déclaration réserve une identité et des budgets ; elle ne charge ni n'exécute le
plugin. La contribution devient visible seulement lorsque le plugin est activé,
compatible, démarré et autorisé par `prompt.segment.register`.

```toml
permissions = ["prompt.segment.register"]

[[prompt_segments]]
name = "node"
summary = "Version Node du projet courant"
timeout_ms = 20
cache_ttl_ms = 30000
```

`name` suit les règles des identifiants de plugin. `summary` est obligatoire et
ne contient pas de caractère de contrôle. `timeout_ms` vaut 25 par défaut et
reste compris entre 1 et 100 ms. `cache_ttl_ms` vaut zéro par défaut et ne peut
pas dépasser cinq minutes.

## Cycle d'une évaluation

```text
PromptContext filtré -> cache valide ? -> requête avec échéance -> valeur typée
                                                    |
                                              délai ou erreur
                                                    v
                                            segment simplement omis
```

Le contexte contient uniquement les données autorisées : dossier sous forme de
token, dernier statut, durée et variables explicitement accordées. Le plugin ne
reçoit ni historique, ni environnement complet, ni handle de terminal.

Un timeout, un crash ou une valeur invalide masque le segment pour ce rendu et
produit un diagnostic limité en fréquence. Le shell ne bloque pas. Le cache est
indexé par plugin, segment, configuration et contexte pertinent ; la
désactivation, un changement de configuration ou un changement de dossier
invalide les entrées concernées.

## Configuration et visibilité

Les options propres au segment restent dans `[plugins.<id>]`. Le plugin les
valide à son démarrage. Le futur ordre du prompt pourra référencer une identité
qualifiée `<plugin-id>.<segment>` ; une référence absente est ignorée avec un
diagnostic, comme une capacité indisponible.

## Rendu séparé

Le résultat transporte une valeur UTF-8 et des rôles sémantiques bornés, jamais
des séquences ANSI, couleurs brutes ou glyphes obligatoires. Le moteur de rendu
choisit la palette, les séparateurs, l'icône ASCII ou Nerd Font et le repli texte.
Le même segment peut ainsi fonctionner dans le CLI, la GUI, un terminal sans
couleurs et les tests de capture.
