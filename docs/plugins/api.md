# Contrat public des plugins

Statut : contrat de conception T102, version `1.0` proposée ; aucune
implémentation n'est encore exportée par le crate.

Ce document complète l'[architecture cible](architecture.md). Il fixe le plus
petit contrat utile avant d'écrire `src/plugin/api.rs`, `context.rs` ou les
erreurs correspondantes. Les noms Rust ci-dessous sont des esquisses de DTO,
pas une promesse d'ABI Rust.

## Promesse de stabilité

L'API publique est un protocole de messages, indépendant du runtime et du
langage. Une implémentation Rust peut utiliser `serde`, un composant WebAssembly
peut utiliser WIT et un processus peut utiliser JSON ; les mêmes champs et
règles s'appliquent.

La version comporte un majeur et un mineur :

- le majeur change lorsqu'un champ, une variante ou une sémantique obligatoire
  devient incompatible ;
- le mineur peut ajouter un champ optionnel, une capacité ou une variante que
  le destinataire est autorisé à ignorer ;
- le manifeste déclare un intervalle de versions accepté ;
- l'activation échoue avant tout démarrage si aucun majeur commun n'existe.

La version `1.0` doit rester expérimentale tant qu'un plugin officiel n'a pas
prouvé chaque type sur Windows, macOS et Linux.

## Enveloppe commune

Chaque appel porte une identité et une limite explicite :

```rust
struct PluginRequest {
    api_version: ApiVersion,
    request_id: RequestId,
    plugin_id: PluginId,
    deadline_ms: u64,
    body: RequestBody,
}

struct PluginResponse {
    api_version: ApiVersion,
    request_id: RequestId,
    body: Result<ResponseBody, PluginError>,
}
```

`RequestId`, `PluginId`, `SessionId` et les tokens de ressources sont des
chaînes opaques. Le plugin ne peut en déduire un PID, un chemin ou un handle.
La réponse doit répéter le `request_id`; une réponse tardive ou inconnue est
ignorée puis comptée comme faute du runtime.

## Contexte exposé

Le contexte est une photographie immuable limitée à l'appel courant :

```rust
struct PluginContext {
    session_id: SessionId,
    working_directory: DisplayPath,
    environment: Vec<EnvironmentValue>,
    terminal: TerminalCapabilities,
    last_status: Option<i32>,
    plugin_config: JsonObject,
}

struct DisplayPath {
    display: String,
    resource: Option<ResourceToken>,
}

struct EnvironmentValue {
    name: String,
    value: String,
}
```

- `display` est une représentation UTF-8 destinée à l'affichage. Elle ne sert
  jamais à reconstruire un chemin par concaténation.
- `resource` n'est présent que si une permission autorise une opération hôte
  sur ce chemin. Le plugin renvoie le token au service concerné.
- `environment` contient uniquement les noms demandés dans le manifeste,
  autorisés puis jugés non sensibles par l'hôte. Il n'existe pas d'opération
  « lire toutes les variables ».
- `TerminalCapabilities` décrit couleurs, icônes et largeur utiles au rendu ;
  il n'expose ni xterm, ni PTY, ni fenêtre Tauri.
- `plugin_config` contient uniquement la section de configuration de ce plugin.

Le contexte n'expose pas l'historique, la ligne en cours d'édition, les autres
sessions, les arguments de lancement, les secrets ni les objets du moteur.

## Capacités de l'hôte

Une capacité doit être déclarée par le plugin et accordée avant l'appel.

| Identifiant | Donnée ou action | Règle initiale |
| --- | --- | --- |
| `context.cwd.read` | Dossier courant affichable | Lecture seule |
| `context.env.read` | Variables nommées | Liste de noms obligatoire |
| `config.read` | Configuration propre au plugin | Lecture seule |
| `command.register` | Contribution de commande | Nom sans conflit natif |
| `prompt.segment.register` | Contribution de segment | Budget court, résultat sans ANSI |
| `completion.register` | Candidats de complétion | Aucun accès disque implicite |
| `output.write` | Sortie contrôlée | Texte ou styles sémantiques uniquement |

`filesystem.read`, `filesystem.write`, `process.spawn` et `network.connect` ne
font pas partie de l'API `1.0`. Leur ajout nécessiterait une nouvelle décision,
des paramètres de portée et une interface utilisateur de consentement.

## Contributions et retours

### Commande

```rust
struct CommandDescriptor {
    id: ContributionId,
    name: String,
    summary: String,
    arguments: Vec<ArgumentDescriptor>,
}

struct ExecuteCommand {
    contribution_id: ContributionId,
    arguments: Vec<String>,
    context: PluginContext,
}

struct CommandResult {
    status: CommandStatus,
    output: Vec<OutputChunk>,
    directory_change: Option<ResourceToken>,
}
```

Les arguments sont déjà parsés par l'hôte. `OutputChunk` distingue sortie
standard et erreur, contient du texte UTF-8 et éventuellement un style
sémantique connu. Les séquences ANSI fournies par un plugin sont traitées comme
du texte. Un changement de dossier utilise un token validé par l'hôte ; le
plugin ne modifie jamais directement le processus.

### Segment de prompt

```rust
struct RenderPromptSegment {
    contribution_id: ContributionId,
    context: PromptPluginContext,
}

struct PromptSegmentResult {
    value: Option<String>,
    style_key: Option<String>,
    icon_key: Option<String>,
    cache_for_ms: Option<u64>,
}
```

Le contexte du prompt est plus petit que `PluginContext` et n'inclut que les
données autorisées demandées par le segment. `None` masque le segment. Le cœur
choisit la palette, l'icône de repli, le séparateur et les codes ANSI.

### Complétion

```rust
struct CompleteInput {
    contribution_id: ContributionId,
    command: String,
    arguments: Vec<String>,
    active_prefix: String,
}

struct CompletionResult {
    candidates: Vec<CompletionCandidate>,
    is_complete: bool,
}
```

Le nombre de candidats, leur longueur et le temps d'appel sont bornés. Un
résultat invalide est rejeté sans supprimer les candidats du cœur.

## Erreurs

```rust
enum PluginErrorKind {
    InvalidRequest,
    PermissionDenied,
    UnsupportedApi,
    InvalidResponse,
    Timeout,
    Cancelled,
    RuntimeUnavailable,
    PluginFailure,
}

struct PluginError {
    kind: PluginErrorKind,
    message: String,
    retryable: bool,
    diagnostic_code: Option<String>,
}
```

`message` est sûr pour l'utilisateur : il ne contient ni secret, ni trace
complète, ni chemin non autorisé. L'hôte ajoute l'identifiant du plugin et le
contexte d'opération à son propre diagnostic. Une erreur de plugin ne devient
jamais un panic du cœur.

| Erreur | Effet hôte |
| --- | --- |
| Requête ou réponse invalide | Refuser l'appel et compter une faute |
| Permission refusée | Ne pas appeler le service ; contribution conservée |
| Délai ou annulation | Ignorer la réponse tardive ; interrompre si possible |
| Runtime indisponible | Masquer les contributions exécutables concernées |
| Échecs répétés | Désactiver le plugin et expliquer la récupération |

## Initialisation et arrêt

Le plugin reçoit d'abord `Initialize` avec la version négociée, ses permissions
accordées et sa configuration. Il retourne la liste exacte de ses contributions
déclarées ; l'hôte vérifie qu'elle correspond au manifeste avant de les rendre
visibles.

`Shutdown` indique une échéance, puis le superviseur annule les requêtes et
arrête le runtime. Un plugin ne peut pas refuser sa désactivation. Aucune
contribution n'est invoquée après son retrait du registre.

## Trait interne de runtime

Le seul trait lié au choix technique reste privé au crate :

```rust
trait PluginRuntime {
    fn start(&mut self, init: Initialize) -> Result<Initialized, RuntimeError>;
    fn call(&mut self, request: PluginRequest) -> Result<PluginResponse, RuntimeError>;
    fn cancel(&mut self, request_id: &RequestId) -> Result<(), RuntimeError>;
    fn stop(&mut self, deadline_ms: u64) -> Result<(), RuntimeError>;
}
```

Ce trait n'est pas l'API des auteurs de plugins et peut évoluer. Les DTO qu'il
transporte, eux, suivent les règles de version ci-dessus.

## Règles de validation de l'implémentation future

- Sérialisation aller-retour de chaque DTO, y compris champs optionnels inconnus.
- Rejet d'un majeur incompatible avant `start`.
- Aucun type de `core`, `tauri`, `portable-pty` ou `rustyline` dans l'API.
- Délai, annulation et taille maximale testés sans horloge réelle fragile.
- Chemins avec espaces, Unicode, LF et CRLF couverts.
- Messages identiques sur Windows, macOS et Linux.
- Fuzzing du parseur de messages et aucune utilisation de `unwrap` sur une
  donnée fournie par un plugin.

T102 s'arrête à ce contrat. T107 définira le manifeste qui demande ces
capacités ; T117 sera la première preuve d'implémentation bout en bout.
