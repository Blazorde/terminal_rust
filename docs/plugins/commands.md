# Commandes de plugins

Une commande de plugin est une contribution déclarée dans le manifeste, puis
validée et publiée par l'hôte seulement si le plugin est activé, compatible et
démarré. Le manifeste fournit aujourd'hui son nom et son résumé ; il ne rend pas
la commande exécutable à lui seul.

## Lien avec le dispatcher

Les commandes natives restent la référence dans `src/app/command.rs`. Chaque
entrée contient le nom utilisé par le dispatcher, la syntaxe et le résumé
utilisés par l'aide, la limite d'arguments et le handler. L'aide et
l'autocomplétion lisent donc la même table que l'exécution.

Le futur adaptateur plugin produira une vue compatible, sans transformer une
fonction externe en pointeur Rust :

```text
entrée utilisateur
  -> registre unifié (natif prioritaire)
  -> validation des arguments
  -> requête ExecuteCommand sérialisée
  -> superviseur avec délai et annulation
  -> résultat typé
  -> rendu stdout/stderr par l'hôte
```

Une commande native gagne toujours un conflit de nom. Deux plugins qui déclarent
le même nom restent en erreur jusqu'à résolution explicite ; l'ordre du système
de fichiers ne choisit jamais le gagnant.

## Aide et autocomplétion

Une contribution n'apparaît dans `help` et dans la complétion de premier mot
qu'après son activation complète. La désactivation ou l'arrêt du runtime retire
les deux vues avant toute nouvelle exécution. Les complétions d'arguments
utiliseront la capacité `completion.register`, un préfixe borné et un délai plus
court que l'exécution d'une commande.

Le nom est limité à 64 caractères ASCII minuscules, chiffres, `-` ou `_`. Le
résumé est obligatoire, sans caractères de contrôle. Les arguments sont transmis
comme une liste déjà parsée ; le plugin ne reçoit jamais la ligne brute.

## Résultat et sécurité

Le résultat distingue statut, blocs stdout/stderr et changement de dossier par
token opaque. Le cœur reste seul responsable de l'affichage et du dossier du
processus. Les séquences ANSI d'un plugin sont du texte, pas du contrôle de
terminal.

Une commande n'obtient que les capacités déclarées dans son manifeste et
accordées par l'hôte. `process.spawn`, l'accès réseau et l'accès arbitraire au
système de fichiers ne font pas partie de l'API v1. Chaque requête reçoit une
échéance ; le superviseur peut l'annuler et retirer toutes les contributions du
plugin. Un crash ou timeout devient un diagnostic de commande et ne ferme pas le
shell.

## Étapes restantes

T104 fixe le contrat et prépare le registre natif, mais n'exécute pas encore de
code tiers. L'enregistrement dynamique attend le runtime isolé et ses tests de
cycle de vie. Cette séparation évite d'exposer une fausse commande dans l'aide
ou la complétion avant qu'elle puisse réellement répondre.
