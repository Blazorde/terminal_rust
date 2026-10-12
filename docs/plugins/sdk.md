# SDK minimal de plugins

Le crate `terminal-rust-plugin-sdk` contient la frontière publique et portable
utilisée par le template. Il ne dépend ni du cœur du shell, ni de Tauri, ni d'un
runtime de plugins. Ses structures sont sérialisables afin qu'un futur adaptateur
puisse les transporter par messages, composant WebAssembly ou processus isolé
sans changer le contrat métier.

Le SDK expose :

- `HOST_API_VERSION` et `ApiVersion` ;
- `CommandRequest`, des arguments déjà parsés et un `PluginContext` filtré ;
- `CommandResponse`, `CommandStatus` et des `OutputChunk` stdout/stderr ;
- `ResourceToken`, qui évite d'exposer directement un chemin hôte ;
- `PluginError`, avec code stable et caractère réessayable ;
- le trait minimal `Plugin` pour tester une implémentation sans transport.

Le trait n'est pas une ABI dynamique et le cœur ne charge pas le `.rlib` du
template. Le futur runtime adapte des messages sérialisés vers cette logique.
Cette contrainte évite de lier la sécurité et la compatibilité à l'ABI Rust.

Voir le [`README` du template](../../plugins/example-plugin/README.md) pour le
copier, le compiler, le tester et faire reconnaître son manifeste localement.
