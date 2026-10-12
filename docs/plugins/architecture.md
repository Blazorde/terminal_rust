# Architecture cible des plugins

Cette architecture concrétise la [vision long terme](../vision-plugins.md), le
[contrat public proposé](api.md) et
l'[ADR-0006](../decisions/0006-plugin-boundary.md). Elle décrit les frontières à
construire ; aucun runtime de code tiers n'est présent dans le MVP actuel.

## Définition

Un plugin Terminal Rust est un paquet identifié et versionné qui déclare une ou
plusieurs contributions conformes à l'API publique. Il n'est ni une référence à
`ShellEngine`, ni un script shell implicitement approuvé, ni une bibliothèque
Rust chargée dans le processus principal.

| Type | Contribution | Service hôte maximal |
| --- | --- | --- |
| Commande | Nom, arguments et résultat de commande | Appel d'une capacité bornée |
| Prompt | Un ou plusieurs `PromptSegment` | Contexte explicitement filtré |
| Thème | Palette, séparateurs et icônes déclaratifs | Aucun effet de bord |
| Intégration | État Git, Docker ou runtime de projet | Adaptateur autorisé et délai court |
| Outil | Dashboard, helper ou action développeur | Capacités listées dans le manifeste |
| Interface | Panneau ou vue déclarative | Modèle de vue validé, jamais objet WebView |

Une même archive peut contenir plusieurs contributions, mais possède une seule
identité, une seule version et une union visible de ses permissions.

## Composants et responsabilités

```text
répertoire de plugins
        │ lecture de manifestes uniquement
        ▼
ManifestLoader ──► ManifestValidator ──► PluginRegistry
                                              │
configuration ──► activation ──► PermissionSet│
                                              ▼
ShellEngine ◄── HostServices ◄── PluginSupervisor ──► PluginRuntime
     │              │                    │                  │
 commandes      DTO publics        délai / arrêt      déclaratif ou
 prompt         et bornés           / diagnostic      runtime isolé
```

- **ManifestLoader** trouve les manifestes dans un emplacement natif connu. Il
  ne charge pas de code et ne suit pas un lien hors de la racine autorisée.
- **ManifestValidator** vérifie schéma, identité, version, compatibilité,
  contributions, chemins relatifs et permissions.
- **PluginRegistry** conserve les versions installées, l'état activé et les
  contributions résolues. Il ne contient aucun handle d'exécution.
- **PermissionSet** est l'intersection des capacités demandées, comprises par
  l'hôte et accordées par l'utilisateur.
- **HostServices** convertit les requêtes publiques en appels bornés vers le
  moteur. C'est l'unique accès d'un plugin aux fonctions de Terminal Rust.
- **PluginSupervisor** applique délais, annulation, taille maximale des messages,
  état de santé et arrêt.
- **PluginRuntime** est un port interne. Une implémentation déclarative peut
  précéder un runtime WASM ou processus sans changer le registre.

## Arborescence future

Le dossier ne sera créé qu'avec le premier contrat exécutable afin d'éviter des
modules vides. La cible est :

```text
src/plugin/
├── mod.rs          # exports internes et assemblage
├── api.rs          # DTO publics et version de protocole
├── manifest.rs     # schéma, parsing et validation pure
├── registry.rs     # découverte résolue et conflits
├── permissions.rs  # capacités demandées et accordées
├── supervisor.rs   # cycle de vie, délais et diagnostics
└── runtime.rs      # trait interne, sans dépendance d'implémentation
```

Les implémentations lourdes éventuelles vivent dans des crates séparées, par
exemple `crates/plugin-runtime-wasm`, afin que le CLI minimal ne les compile
pas lorsqu'elles sont désactivées.

## Installation et stockage

L'installation distante n'appartient pas au premier incrément. Un paquet local
est copié de façon atomique dans le dossier de données déterminé par la
plateforme, jamais à partir d'une concaténation de `HOME` :

```text
<data_dir>/terminal-rust/plugins/<plugin-id>/<version>/
├── terminal-rust-plugin.toml
├── assets/
└── component.wasm ou executable futur
```

Le registre actif référence un identifiant et une version exacte. Installer ne
signifie pas activer. Une mise à jour ajoute une version, la valide, puis change
la référence active ; l'ancienne version reste disponible pour un rollback
jusqu'au nettoyage explicite.

## Cycle de vie

```text
absent -> découvert -> validé -> installé -> activé -> démarré
                    \-> refusé              \-> en erreur
démarré -> suspendu -> arrêté -> désactivé
```

1. **Découvrir** lit le manifeste et calcule l'identité du contenu.
2. **Valider** vérifie le schéma sans exécuter le paquet.
3. **Installer** écrit dans un dossier temporaire puis effectue un remplacement
   atomique lorsque la plateforme le permet.
4. **Activer** résout versions, conflits et permissions.
5. **Démarrer** est paresseux, au premier appel d'une contribution.
6. **Superviser** borne chaque appel et attribue tout diagnostic au plugin.
7. **Désactiver** retire d'abord les contributions, annule les appels, arrête le
   runtime puis persiste l'état.

Un arrêt incomplet ne laisse pas la contribution enregistrée. Au prochain
démarrage, un plugin précédemment fautif reste désactivé jusqu'à une action
explicite.

## Frontière avec le cœur

Le cœur expose des services et des données, pas ses structures internes. Les
DTO futurs contiendront des chaînes UTF-8, nombres bornés, identifiants stables,
listes et objets versionnés. Ils n'exposeront pas :

- `&mut ShellEngine`, `PathBuf` brut ou type d'erreur privé ;
- handle de processus, PTY, fenêtre Tauri ou canal d'événement ;
- variable d'environnement complète ou historique de commandes ;
- callback conservé au-delà de la requête.

Une commande de plugin reçoit des arguments déjà parsés et retourne un résultat
typé. Un segment reçoit un `PromptContext` filtré et retourne une valeur sans
séquence ANSI. Une complétion reçoit un préfixe et un contexte minimal. Le cœur
reste responsable du rendu, de la sortie et de la mutation du dossier courant.

## Résolution et compatibilité

- Un identifiant utilise une forme stable de type `éditeur.nom`.
- Le manifeste déclare sa version SemVer, la version de schéma et une plage
  d'API hôte compatible.
- Une version majeure de l'API retire ou change un champ ; une mineure ajoute
  uniquement des champs ou capacités optionnels.
- Un hôte ignore une contribution optionnelle inconnue, mais refuse une
  permission ou une exigence obligatoire inconnue.
- Une commande native gagne toujours un conflit de nom. Deux plugins en conflit
  sont désactivés tant qu'une priorité explicite n'est pas configurée.
- L'ordre de découverte du système de fichiers ne décide jamais du gagnant.

## Compatibilité multiplateforme

Le manifeste et les messages ne contiennent pas de séparateur de chemin propre
à un OS. Les chemins du paquet sont relatifs, normalisés par l'hôte et ne
peuvent pas remonter avec `..`. Un plugin exécutable déclare ses artefacts par
triplet cible ; un composant WASM reste commun lorsqu'il n'importe aucune
capacité spécifique.

Chaque évolution du contrat doit être testée au minimum avec : chemins contenant
des espaces, LF et CRLF, nom Unicode, plugin absent, manifeste inconnu, version
incompatible, permission refusée, délai dépassé et arrêt pendant un appel. La
matrice Windows/macOS/Linux reste obligatoire avant de déclarer un runtime
portable.

## Ordre d'implémentation

1. T102 définit uniquement les DTO et règles de compatibilité.
2. T103 ajoute la configuration sans découverte de code.
3. T106 et T107 livrent le registre local, `plugins list` et le manifeste validé.
4. T104 et T105 branchent une commande et un segment déclaratifs.
5. T108 puis T117 fournissent le SDK minimal et un plugin officiel.
6. Un ADR ultérieur choisit WASM ou processus après un spike mesuré.

Cette séquence garde chaque PR supprimable et le shell fonctionnel sans plugin.
