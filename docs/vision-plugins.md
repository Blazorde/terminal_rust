# Vision d'un Terminal Rust extensible

Statut : vision long terme, sans runtime de plugins dans le MVP actuel.

La frontière technique retenue est détaillée dans
[l'architecture des plugins](plugins/architecture.md) et
[l'ADR-0006](decisions/0006-plugin-boundary.md).

## Intention

Terminal Rust doit pouvoir évoluer d'un mini-shell pédagogique vers une
plateforme personnalisable sans abandonner ses garanties actuelles : moteur
Rust partagé, comportements explicites, portabilité et dégradation propre. Le
but n'est pas de copier Neovim, mais de reprendre quatre idées qui expliquent
son extensibilité : une API stable, des points d'extension nommés, une
configuration composable et un chargement paresseux.

Neovim distingue les fonctions et événements de son API, ajoute les nouvelles
capacités sans obliger les anciens clients à les utiliser et expose commandes,
raccourcis et événements par des contrats dédiés. Son `runtimepath` ordonne la
découverte des modules, tandis que son guide recommande de garder
l'initialisation minimale et de différer le chargement du code coûteux. Ces
principes sont documentés dans le [contrat de l'API
Neovim](https://neovim.io/doc/user/api/#api-contract), le [guide
Lua](https://neovim.io/doc/user/lua-guide.html) et le [guide des plugins
Lua](https://neovim.io/doc/user/lua-plugin.html).

Terminal Rust doit toutefois être plus restrictif : un terminal manipule des
processus, des chemins et potentiellement des secrets. La commodité d'un script
arbitraire ne doit jamais devenir une permission implicite.

## Modèle mental cible

```text
configuration utilisateur
        │
        ▼
registre de plugins ──► manifeste validé ──► permissions accordées
        │                                      │
        ▼                                      ▼
adaptateur de runtime ◄── messages typés ── processus ou sandbox isolée
        │
        ├── commandes
        ├── segments de prompt
        ├── complétions
        ├── thèmes et raccourcis
        └── événements de session
```

Le moteur ne charge pas directement une bibliothèque dynamique non fiable. Un
adaptateur transforme les appels du plugin en requêtes typées vers les mêmes
services que le CLI et la GUI. Le registre sait ce qui est installé et activé ;
le runtime sait seulement exécuter un plugin déjà validé.

## Points d'extension réalistes

| Extension | Contrat envisagé | Phase |
| --- | --- | --- |
| Thème | Palette et styles déclaratifs, sans code | Post-MVP proche |
| Commande utilisateur | Nom, schéma d'arguments et résultat typé | Post-MVP |
| Segment de prompt | Contexte borné vers `PromptSegment` | Post-MVP |
| Complétion | Préfixe et contexte vers candidats typés | Post-MVP |
| Raccourci | Combinaison vers commande déclarée | Post-MVP |
| Événement | Abonnement à une liste fermée d'événements de session | Long terme |
| Panneau GUI | Vue déclarative, sans accès direct à la WebView | Long terme |
| Outil développeur | Service explicitement autorisé et annulable | Long terme |

Les premières extensions doivent être déclaratives. Elles permettent de
stabiliser le manifeste, la configuration et les erreurs avant de choisir un
runtime de code. Les commandes et segments officiels servent ensuite de tests
de référence pour l'API publique.

## Ce qu'un plugin ne peut pas faire par défaut

- lire ou écrire un chemin arbitraire ;
- lire l'environnement complet, l'historique ou les secrets du processus ;
- ouvrir le réseau, lancer un processus ou invoquer un shell ;
- accéder aux handles PTY, Tauri ou aux objets internes du moteur ;
- enregistrer une commande sous le nom d'une commande native ;
- bloquer le rendu du prompt ou la boucle interactive ;
- modifier sa propre liste de permissions ;
- rester actif après désactivation ou échec de son arrêt.

Une capacité sensible est refusée par défaut, déclarée dans le manifeste puis
accordée explicitement. Les chemins autorisés sont normalisés avec `Path` et
`PathBuf`, limités à une racine et vérifiés après résolution. Les appels ont un
délai, une annulation et des quotas. Un panic, un dépassement ou une sortie
invalide désactive le plugin concerné sans interrompre le shell.

## Contrats à stabiliser avant le runtime

1. **Identité** — identifiant global, version SemVer, auteur et provenance.
2. **Compatibilité** — plage de versions de l'hôte et version exacte de l'API.
3. **Capacités** — liste fermée, paramètres minimaux et justification lisible.
4. **Entrées/sorties** — DTO sérialisables, versionnés et sans type interne Rust.
5. **Cycle de vie** — découverte, validation, activation, appel, désactivation et
   désinstallation déterministes.
6. **Erreurs** — catégories stables, diagnostic utilisateur et repli défini.
7. **Observabilité** — temps d'appel, échecs et plugin responsable, sans contenu
   de commande ni secret dans les logs.

Le transport précis reste une décision ultérieure. WASM avec une interface WIT
ou un processus enfant échangeant du JSON sont préférables à une ABI Rust
dynamique : ils isolent mieux les fautes et évitent de lier un plugin à la
version du compilateur. T101 doit comparer ces options avant tout prototype.

## Chargement et performance

La découverte lit uniquement les manifestes. Un plugin n'est démarré qu'à la
première capacité utilisée, sauf activation explicite au démarrage. Comme dans
le modèle Neovim, une commande ou un événement peut être enregistré sans
charger tout le module. Les segments de prompt ont un budget particulièrement
court ; leur dernier résultat peut être mis en cache et tout dépassement doit
produire une absence de segment, jamais un prompt bloqué.

L'ordre est déterministe : cœur natif, plugins officiels, plugins utilisateur,
puis surcharge explicite autorisée. Deux fournisseurs ne peuvent pas revendiquer
silencieusement le même identifiant.

## Roadmap séparée du MVP

### MVP actuel

- moteur et commandes internes compilés avec l'application ;
- prompt segmenté et thèmes intégrés ;
- configuration TOML validée ;
- aucune exécution de code tiers ni installation automatique.

### Post-MVP

- T101 à T103 : architecture, API et configuration ;
- T104 et T105 : prototypes bornés de commandes et segments ;
- T106 et T107 : registre local et manifeste ;
- T108 et T117 : SDK minimal et premier plugin officiel ;
- T109 : commandes utilisateur déclaratives.

### Long terme

- scripting et automatisation contrôlés ;
- panneaux, splits et outils développeur ;
- permissions, compatibilité et migration ;
- galerie locale vérifiée, sans installation silencieuse.

Cette roadmap n'engage ni le langage du runtime ni une marketplace distante.
Le [comparatif des runtimes de scripting](research/scripting-runtime.md) propose
Rhai pour une expérimentation future de scripts locaux approuvés, sans modifier
la préférence WebAssembly de l'ADR-0006 pour les plugins tiers non fiables.
Chaque phase doit rester supprimable sans casser le shell natif.

## Critères de décision pour la suite

Une proposition de runtime n'est acceptable que si elle prouve sur Windows,
macOS et Linux : isolation d'un crash, refus d'une permission, délai d'appel,
désactivation à chaud, compatibilité de versions et coût de démarrage. Sans ces
preuves, Terminal Rust conserve des extensions déclaratives uniquement.
