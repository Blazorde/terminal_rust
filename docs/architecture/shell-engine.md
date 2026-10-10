# API du moteur de shell

`ShellEngine` est la frontière Rust partagée entre le binaire CLI et les futurs
adaptateurs desktop. Il reçoit une ligne déjà saisie ou une liste d'arguments,
exécute le registre de commandes existant et retourne un résultat sans imposer
la manière de l'afficher.

```rust
use terminal_rust::{ExecutionStatus, ShellEngine};

let mut engine = ShellEngine::new();
let result = engine.execute_line("pwd");

if result.status == ExecutionStatus::Failed {
    eprintln!("{}", result.error.as_deref().unwrap_or("unknown error"));
} else {
    print!("{}", result.stdout);
}
```

## Contrat

- `status` distingue la poursuite, la sortie demandée et l'échec récupérable.
- `stdout` et `stderr` sont séparés et ne sont pas écrits par le moteur.
- `error` contient le diagnostic de dispatch ou de commande lorsque le statut
  vaut `Failed`.
- `current_dir` reflète le dossier courant après l'exécution lorsqu'il est
  accessible.
- `directory_change` contient les chemins avant et après un `cd` réussi.

Le moteur conserve l'état de session nécessaire à `cd -`. Une instance doit
donc appartenir à une seule session et ne doit pas être partagée entre deux
terminaux concurrents. La future couche PTY reste responsable des commandes
externes, des confirmations interactives et du cycle de vie des processus.

## Compatibilité

L'API utilise `PathBuf` pour les chemins et ne dépend d'aucune commande Unix ou
PowerShell. Les consommateurs ne doivent pas analyser la sortie textuelle pour
détecter un changement de dossier : `directory_change` est le contrat prévu à
cet effet.
