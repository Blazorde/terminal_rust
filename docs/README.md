# Documentation de Terminal Rust

Ce dossier est la porte d'entrée de la documentation technique. Le README à la
racine reste orienté utilisateur ; les détails de conception et de contribution
vivent ici.

## Parcours conseillé

| Besoin | Document |
| --- | --- |
| Comprendre le système | [Vue d'ensemble](architecture/overview.md) |
| Utiliser le moteur partagé | [API du moteur de shell](architecture/shell-engine.md) |
| Préparer l'interface desktop | [Architecture GUI](architecture/desktop-gui.md) |
| Comprendre la vision des plugins | [Vision d'un Terminal Rust extensible](vision-plugins.md) |
| Préparer l'architecture des plugins | [Architecture cible des plugins](plugins/architecture.md) |
| Examiner le futur contrat public des plugins | [API de plugins proposée](plugins/api.md) |
| Configurer les plugins | [Configuration des plugins](plugins/configuration.md) |
| Écrire un manifeste de plugin | [Format du manifeste](plugins/manifest.md) |
| Comprendre la découverte locale | [Registre local des plugins](plugins/registry.md) |
| Concevoir une commande plugin | [Commandes de plugins](plugins/commands.md) |
| Développer et valider l'interface desktop | [Guide desktop](development/desktop.md) |
| Consulter l'audit qualité GUI | [Audit accessibilité, responsive et performance](development/gui-quality-audit.md) |
| Concevoir le futur prompt | [Moteur de prompt](architecture/prompt-engine.md) |
| Vérifier la portabilité | [Compatibilité multiplateforme](architecture/cross-platform.md) |
| Comprendre une décision | [Registre des ADR](decisions/README.md) |
| Personnaliser le prompt | [Personnalisation](customization.md) |
| Configurer le shell | [Configuration utilisateur](configuration.md) |
| Créer un thème | [Format des thèmes](themes.md) |
| Contribuer | [Guide de contribution](development/contributing.md) |
| Tester une évolution | [Stratégie de tests](development/testing.md) |
| Valider le MVP | [Checklist qualité MVP](development/mvp-quality-checklist.md) |
| Installer Terminal Rust | [Installation](install.md) |
| Résoudre un problème | [Dépannage](troubleshooting.md) |
| Produire un binaire | [Builds de distribution](release.md) |
| Traiter un ticket | [Workflow des agents](agents/ticket-workflow.md) |
| Configurer GitHub | [Workflows et protections](workflows/README.md) |

## Principes

- La documentation décrit l'état réel ou indique clairement qu'une proposition
  est future.
- Une décision structurante reçoit un ADR plutôt qu'une note isolée dans une
  PR.
- Les exemples restent exécutables sur Windows, macOS et Linux, ou signalent
  explicitement leur plateforme.
- Les chemins internes sont relatifs à la racine du dépôt afin que les liens
  fonctionnent sur GitHub et dans les éditeurs.
