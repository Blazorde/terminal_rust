# Commandes utilisateur

Les commandes utilisateur créent des raccourcis portables sans modifier le
binaire. Chaque commande est une table TOML structurée ; Terminal Rust ne passe
jamais sa valeur à `sh -c`, `cmd /C` ou PowerShell.

## Alias d'une commande interne

```toml
[commands.git-summary]
kind = "builtin"
target = "git"
args = ["status", "--short", "--branch"]
append_args = false
description = "Show a concise Git summary"
```

`target` doit être une commande interne existante. Les commandes natives ont
toujours priorité : déclarer `help`, `git` ou tout autre nom natif comme commande
personnelle désactive toutes les commandes personnalisées avec un diagnostic.

## Processus externe

```toml
[commands.serve]
kind = "process"
target = "npm"
args = ["run", "dev"]
append_args = true
description = "Start the project development server"
```

Le programme est lancé directement avec `std::process::Command`. Chaque élément
de `args` reste un argument distinct. Les opérateurs `|`, `>`, `&&`, les
substitutions et les variables de shell ne sont jamais interprétés. Le programme
hérite toutefois du dossier courant et de l'environnement de Terminal Rust : une
commande configurée peut donc lire des secrets accessibles au processus et doit
être considérée comme du code approuvé par l'utilisateur.

Si `append_args` vaut `true`, les arguments saisis après le nom sont ajoutés tels
quels. S'il vaut `false`, tout argument supplémentaire produit une erreur au lieu
d'être ignoré. La sortie standard, la sortie d'erreur et le code de retour du
processus sont conservés.

## Découverte et erreurs

`commands list` affiche les commandes configurées, leur type, leur cible et leur
description. Elles sont aussi proposées par l'autocomplétion interactive.

Un nom contient 1 à 64 lettres ASCII minuscules, chiffres, tirets ou tirets bas,
et commence et finit par une lettre ou un chiffre. Sont refusés au chargement :

- les conflits avec une commande native ;
- une cible interne inconnue ;
- une cible vide ;
- un octet NUL dans une cible ou un argument ;
- une description contenant un caractère de contrôle.

Une configuration invalide ne bloque pas le shell : le diagnostic est affiché,
les commandes personnelles sont désactivées et les commandes natives restent
disponibles.

Les processus externes dépendent naturellement des outils installés et de leur
nom sur Windows, macOS ou Linux. Pour un alias portable, préférer une commande
interne. Pour un processus, documenter les prérequis et éviter les chemins codés
en dur avec `/` ou `\\`.
