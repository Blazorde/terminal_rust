# ADR-0006 — Frontière de plugins déclarative et isolable

- Statut : Acceptée
- Date : 2026-10-12
- Décideurs : équipe Terminal Rust

## Contexte

La vision T100 prévoit des commandes, segments de prompt, complétions, thèmes
et outils extensibles. Une extension s'exécute au contact d'un terminal réel :
elle peut ralentir le prompt, lire des chemins, voir des commandes sensibles ou
faire tomber le processus hôte. La frontière doit donc être stable avant le
choix d'un langage ou d'un runtime.

Trois contraintes structurent la décision :

- Windows, macOS et Linux doivent partager le même manifeste et le même contrat ;
- le shell doit rester utilisable lorsqu'un plugin manque, expire ou échoue ;
- un plugin tiers ne doit recevoir ni référence Rust interne, ni handle PTY, ni
  accès implicite au système.

Le chargement d'une bibliothèque compilée avec l'ABI Rust n'est pas un contrat
portable : la [référence Rust](https://doc.rust-lang.org/reference/items/external-blocks.html#abi)
précise que cette ABI n'offre aucune garantie de stabilité. Le Component Model
WebAssembly propose au contraire des interfaces typées WIT, et Wasmtime refuse
par défaut l'accès aux ressources système ; son adoption représente toutefois
un coût et doit être mesurée. Voir la [présentation du Component
Model](https://component-model.bytecodealliance.org/) et son [exécution avec
Wasmtime](https://component-model.bytecodealliance.org/running-components/wasmtime.html).

## Décision

Adopter une architecture en deux paliers autour d'une même frontière de
messages versionnés.

1. Le premier palier accepte uniquement des plugins déclaratifs : manifestes,
   thèmes, raccourcis et commandes composées à partir de capacités de l'hôte.
   Aucun code tiers n'est exécuté.
2. Le second palier, futur, exécute du code derrière un trait interne
   `PluginRuntime`. Le runtime ne reçoit et ne retourne que les DTO publics de
   l'API de plugins.
3. Le cœur possède le registre, la validation des manifestes, la résolution des
   conflits, les permissions, les délais et la conversion vers les services du
   moteur. Le runtime ne contourne jamais cette couche.
4. Les bibliothèques dynamiques utilisant l'ABI Rust sont exclues pour les
   plugins tiers. Un prototype ultérieur comparera un composant WebAssembly à
   un processus enfant utilisant un protocole sérialisé. Le choix final exige
   des preuves de sécurité, portabilité et performance.
5. La découverte ne démarre rien. L'activation est explicite, réversible et
   enregistrée par identifiant stable. Un plugin incompatible ou invalide reste
   désactivé avec un diagnostic utile.

La frontière logique est indépendante du runtime :

```text
ShellEngine / Prompt / Completion
              │
       services hôte bornés
              │
       API de plugins DTO
              │
 registre ─ permissions ─ superviseur
              │
       PluginRuntime (port)
              │
   déclaratif | WASM | processus isolé
```

## Options considérées

| Option | Avantages | Limites | Décision |
| --- | --- | --- | --- |
| Formats déclaratifs | Simples, auditables, portables, aucun code arbitraire | Capacités volontairement limitées | Retenus en premier |
| Crates liées à la compilation | Typage Rust et coût d'appel faible | Nécessitent de reconstruire l'application ; plugins non indépendants | Réservées aux extensions officielles internes |
| Bibliothèques dynamiques Rust | Chargement natif et appels directs | ABI instable, crash dans l'hôte, packaging propre à chaque OS | Rejetées pour les tiers |
| Processus enfant + messages | Isolation des crashs, langage libre, débogage simple | Un exécutable tiers conserve les droits de l'utilisateur ; sandbox OS différente | Candidat à prototyper |
| WebAssembly Component Model | Interface WIT, refus de ressources par défaut, binaire portable | Runtime plus lourd, outillage WASI encore évolutif | Candidat privilégié, à mesurer |
| Interpréteur embarqué | Configuration expressive et écosystème familier | Surface d'attaque et sandbox à construire | Non retenu avant stabilisation de l'API |

## Conséquences

### Positives

- Le contrat public peut être testé avant le choix du runtime.
- Les premières extensions ne peuvent pas exécuter arbitrairement un processus
  ou lire le disque.
- Le cœur reste indépendant de Wasmtime, Lua ou d'un protocole particulier.
- Un runtime peut être remplacé sans modifier les commandes, le prompt ou la
  configuration utilisateur.
- La même découverte et les mêmes diagnostics s'appliquent sur les trois OS.

### Négatives

- Les premiers plugins seront moins puissants qu'un script Neovim.
- Une couche de DTO et de conversion ajoute du code par rapport à des traits
  Rust exposés directement.
- Le choix WASM/processus reste une décision ouverte jusqu'au spike ; aucun
  ticket ne doit annoncer un runtime définitif avant ces mesures.
- L'isolation d'un processus enfant ne constitue pas à elle seule une sandbox.

## Validation

Les tickets suivants doivent démontrer la décision par incréments :

- T102 versionne l'API et interdit les types internes dans le contrat public ;
- T103 définit une configuration qui désactive proprement les entrées invalides ;
- T106 implémente un registre sans exécuter le contenu découvert ;
- T107 valide le manifeste, les permissions et la compatibilité ;
- T117 fournit un plugin officiel testant activation, erreur et désactivation ;
- tout runtime de code prouve refus de permission, délai, annulation, crash
  isolé et fonctionnement Windows/macOS/Linux.

La suite obligatoire du dépôt reste verte sans plugin installé. Un plugin
absent ou refusé ne doit jamais empêcher le CLI ou la GUI de démarrer.
