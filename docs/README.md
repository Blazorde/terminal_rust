# Documentation de Terminal Rust

Ce dossier est la porte d'entrée de la documentation technique. Le README à la
racine reste orienté utilisateur ; les détails de conception et de contribution
vivent ici.

## Parcours conseillé

| Besoin | Document |
| --- | --- |
| Comprendre le système | [Vue d'ensemble](architecture/overview.md) |
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
