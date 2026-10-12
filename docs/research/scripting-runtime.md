# Recherche — runtime de scripting

- Statut : recommandation long terme, non implémentée
- Date : 2026-10-12
- Périmètre : scripts locaux d'automatisation, distincts des plugins tiers

## Conclusion

Rhai est la piste favorite pour un futur prototype de scripts **locaux et
explicitement approuvés**. Son intégration Rust est directe, son environnement
n'accède à rien que l'hôte n'enregistre, et son moteur expose des limites de
profondeur, appels, opérations, variables, fonctions, modules, chaînes, tableaux
et maps. Cette recommandation ne vaut pas promesse de sandbox contre du code
hostile : Rhai s'exécute dans le processus et toute fonction Rust exposée élargit
ses pouvoirs.

WebAssembly avec le Component Model reste le candidat privilégié par
[l'ADR-0006](../decisions/0006-plugin-boundary.md) pour les plugins tiers non
fiables. Lua n'est pas retenu pour le premier prototype : il est excellent comme
langage embarqué, mais l'hôte devrait construire lui-même sa politique de
sandbox et maintenir une intégration C supplémentaire.

Le scripting vient **après** le manifeste, les permissions, la compatibilité et
le superviseur de plugins. Les commandes structurées de T109 couvrent entre-temps
les automatismes simples sans interpréteur.

## Comparaison

| Critère | Lua 5.4 | Rhai | WebAssembly + Wasmtime |
| --- | --- | --- | --- |
| Expérience débutant | Langage compact et très documenté | Syntaxe proche de Rust/JavaScript, intégration Rust simple | Nécessite compilation et outillage ; ce n'est pas un script édité puis lancé directement |
| Intégration | API C à adapter et distribuer | Crate Rust embarqué | Runtime et génération d'interfaces WIT |
| Isolation initiale | Dépend entièrement des bibliothèques ouvertes par l'hôte | Aucun accès externe sans fonction enregistrée, mais même processus | Mémoire isolée et fonctions uniquement importées |
| Limites de ressources | Allocateur et hooks à assembler dans l'hôte | Limites natives d'opérations et de tailles | Fuel, epochs, stack et `ResourceLimiter` |
| Contrat typé | Tables/valeurs dynamiques | Types dynamiques convertis côté Rust | Interfaces WIT versionnables |
| Portabilité | C ISO largement portable | Rust pur, mêmes cibles que l'hôte | Composant portable, runtime plus lourd |
| Cas retenu | Non retenu au premier palier | Prototype de scripts locaux approuvés | Plugins tiers isolés, après spike |

## Éléments vérifiés

### Lua

Lua est conçu comme bibliothèque embarquable et son hôte choisit les fonctions
C enregistrées. La bibliothèque standard inclut toutefois entrées/sorties,
système d'exploitation, chargement de modules et débogage. Un hôte sûr devrait
donc éviter `luaL_openlibs` et ouvrir seulement un sous-ensemble. La compatibilité
binaire de l'API et des chunks précompilés n'est pas garantie entre versions.

Sources officielles : [manuel Lua 5.4 — API et bibliothèques](https://www.lua.org/manual/5.4/manual.html#4),
[bibliothèques standard](https://www.lua.org/manual/5.4/manual.html#6),
[incompatibilités](https://www.lua.org/manual/5.4/manual.html#8).

### Rhai

Rhai décrit son moteur comme isolé de l'environnement tant que l'hôte ne lui
enregistre pas d'accès. Il recommande un moteur immutable et fournit des limites
explicites pour empêcher récursion, calcul ou allocations non bornés. Ces garde-
fous peuvent être désactivés par la feature `unchecked`, qui serait interdite.

Sources officielles : [sandbox Rhai](https://rhai.rs/book/safety/sandbox.html),
[protection et limites](https://rhai.rs/book/safety/index.html),
[options de limites du moteur](https://rhai.rs/book/engine/options.html#safety-limits).

### WebAssembly

Wasmtime vise l'exécution de code non fiable dans une sandbox ; le guest ne voit
que les imports fournis. Le Component Model décrit ses contrats avec WIT. Le
runtime offre des limites de mémoire, fuel déterministe et interruption par
epochs. Sa documentation précise que les appels hôte bloquants exigent encore
une gestion asynchrone et des délais côté embedder.

Sources officielles : [sécurité Wasmtime](https://docs.wasmtime.dev/security.html),
[limites et interruption](https://docs.wasmtime.dev/api/wasmtime/struct.Config.html),
[interfaces WIT](https://component-model.bytecodealliance.org/design/wit.html).

## Prototype Rhai autorisable

Un spike séparé du terminal principal devra prouver les points suivants avant
toute dépendance de production :

1. feature Cargo désactivée par défaut ;
2. aucun package, fichier, environnement, réseau, processus ou horloge exposé ;
3. API limitée aux DTO du SDK et à des fonctions pures ;
4. limites d'opérations, profondeur, appels, variables et tailles non nulles ;
5. durée murale supervisée hors du moteur ;
6. sortie bornée et sans séquence ANSI interprétée ;
7. cache compilé invalidé par contenu et version d'API ;
8. erreurs, panics de fonctions hôte et épuisement des quotas testés ;
9. mesures de taille binaire, démarrage et latence sur Windows, macOS et Linux ;
10. aucune activation implicite : chemin explicite et consentement utilisateur.

## Critères d'abandon

Le prototype Rhai sera rejeté si une limite ne peut pas interrompre un calcul,
si une fonction hôte peut contourner les capacités, si les panics traversent la
frontière, ou si le coût dépasse celui d'une commande structurée pour les cas
visés. Dans ce cas, l'automatisation restera déclarative et les extensions de
code utiliseront uniquement le futur runtime WebAssembly/processus évalué par
l'ADR-0006.
