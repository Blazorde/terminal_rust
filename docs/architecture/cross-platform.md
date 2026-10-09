# Compatibilité multiplateforme

Le projet vise Windows, macOS et Linux dès la conception. « Compile partout »
ne suffit pas : les chemins, terminaux et outils disponibles diffèrent.

## Règles communes

- Construire les chemins avec `Path` et `PathBuf`.
- Utiliser `env::current_dir`, `env::var_os` et les API `std::fs` plutôt que des
  commandes système.
- Accepter `HOME` sur Unix et `USERPROFILE` sur Windows.
- Ne pas analyser une sortie localisée quand un code de retour ou un format
  stable existe.
- Masquer les erreurs attendues des outils optionnels ; conserver les erreurs
  utiles pour les actions demandées explicitement par l'utilisateur.

## Terminaux

Les séquences ANSI simples sont utilisables sur les terminaux modernes des trois
plateformes. Toute fonctionnalité avancée devra toutefois prévoir :

- un mode sans couleur ;
- une alternative sans icône ou Nerd Font ;
- une largeur calculée sans supposer qu'un octet vaut une colonne ;
- un repli texte si la capacité du terminal n'est pas connue.

## Processus externes

Git est une dépendance optionnelle. Un appel externe doit :

1. transmettre les arguments séparément à `Command` ;
2. éviter de construire une ligne destinée à un shell ;
3. fixer explicitement le dossier de travail ou utiliser `git -C` ;
4. borner à terme son coût par un cache ou un délai ;
5. convertir « exécutable absent » en capacité indisponible quand l'appel sert
   uniquement à enrichir le prompt.

## Matrice de validation

| Risque | Test attendu |
| --- | --- |
| Séparateurs de chemins | chemins construits avec `PathBuf` |
| Espaces dans un chemin | dossier temporaire contenant un espace |
| Git absent | programme inexistant, aucun panic |
| Hors dépôt | résultat négatif silencieux |
| CRLF/LF | parsing indépendant de la fin de ligne |
| ANSI indisponible | futur mode texte sans décoration |

La CI doit au minimum garder des jobs Windows et Ubuntu. Un job macOS est une
étape de roadmap prioritaire avant la première version du moteur de prompt.
