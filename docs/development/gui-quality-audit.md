# Audit qualité de l'interface desktop

Statut : audit T124 réalisé sous Windows le 11 octobre 2026 ; preuves macOS et
Linux en attente d'un runner disponible.

## Résultat

| Dimension | Score | Preuve principale |
| --- | ---: | --- |
| Accessibilité | 4/4 | Navigation clavier, dialogue modal borné, retour de focus et contrastes AA |
| Performance | 3/4 | Mesures reproductibles et découpage du bundle ; mémoire WebView encore élevée |
| Responsive | 4/4 | Aucun débordement à 1280×720, 560×400 et 320×240 |
| Thèmes | 4/4 | Les trois palettes dépassent 4,5:1 pour tous les textes |
| Intégrité | 4/4 | Détecteur Impeccable sans résultat et aucune donnée simulée |
| **Total** | **19/20** | **Excellent, validation des deux autres OS encore requise** |

L'audit initial a trouvé trois problèmes, tous corrigés dans la même passe :

- `--faint` descendait jusqu'à 2,98:1, sous WCAG AA ;
- le focus pouvait quitter le tiroir de réglages et n'était pas rendu au bouton
  d'ouverture ;
- la réduction de mouvement reposait sur une règle globale à `0,01 ms` au lieu
  de replis explicites.

Le tiroir est désormais un dialogue modal : le contenu arrière devient
`inert`, `Tab` et `Maj+Tab` restent dans le panneau, `Échap` le ferme et rend le
focus au bouton **Réglages**. La préférence de mouvement désactive uniquement
le rail d'activité, la respiration de connexion, la transition du tiroir et le
clignotement du curseur xterm. Les changements de statut restent visibles.

## Contrastes

`npm run audit:ui --prefix desktop` lit les vrais tokens CSS et échoue sous
4,5:1 pour le texte, ou sous 3:1 pour l'indicateur de focus.

| Thème | Texte | Secondaire | Faible | Accent | Erreur | Focus |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Graphite | 13,94 | 6,49 | 5,07 | 6,36 | 7,42 | 10,28 |
| Minuit | 14,85 | 6,99 | 5,22 | 10,69 | 7,82 | 14,08 |
| Papier | 13,55 | 5,13 | 4,61 | 4,54 | 5,01 | 7,92 |

Le contrôle vérifie aussi le repli de mouvement ciblé et le contrat plein
écran. Le détecteur mécanique Impeccable a retourné `[]` après les corrections.

## Responsive

La WebView a été observée avec les dimensions suivantes :

| Vue | Contenu rendu | Débordement | Résultat |
| --- | --- | --- | --- |
| 1280×720 | terminal 1280×720 à l'origine 0,0 | aucun | conforme |
| 560×400 | taille native minimale déclarée par Tauri | aucun | conforme |
| 320×240 | stress inférieur au minimum natif | aucun | conforme |

À 560×400, le tiroir reste défilable et conserve 100 % de ses commandes. La
barre d'onglets défile horizontalement de façon indépendante lorsque le nombre
de sessions dépasse l'espace disponible.

## Mesures Windows

Machine de référence : Windows 11 Professionnel `10.0.26200`, Intel Core Ultra
9 185H (22 processeurs logiques), 31,5 Gio, Rust 1.95.0 et Node 24.11.1. Les
mesures utilisent une build `--release` et trois essais consécutifs.

### Processus GUI

Le démarrage va de `Start-Process` jusqu'à la présence d'une fenêtre répondante.
La mémoire est la somme des working sets du processus Tauri, du shell enfant et
des processus WebView2, une seconde après l'ouverture.

| Essai | Démarrage | Processus | Mémoire |
| ---: | ---: | ---: | ---: |
| 1, froid | 541,1 ms | 10 | 409,5 Mio |
| 2, chaud | 174,4 ms | 10 | 406,7 Mio |
| 3, chaud | 184,8 ms | 10 | 406,6 Mio |

Le premier démarrage reste sous 600 ms sur cette machine. La mémoire reflète
principalement l'isolation de WebView2 ; elle constitue la référence à suivre,
pas une preuve transférable à WebKitGTK ou WKWebView.

### PTY et commande

Le probe ignoré par défaut ouvre un vrai PTY, attend le premier frame OSC 777,
exécute `pwd`, attend le frame suivant et ferme la session :

```bash
cargo test -p terminal-rust-desktop --test pty_shell \
  desktop_performance_probe_reports_three_trials -- --ignored --nocapture
```

| Essai | Premier prompt | Aller-retour `pwd` |
| ---: | ---: | ---: |
| 1 | 70,360 ms | 0,566 ms |
| 2 | 71,799 ms | 0,850 ms |
| 3 | 86,049 ms | 0,660 ms |

Le probe est Rust et `portable-pty` uniquement ; la même commande doit être
rejouée sans modification sur macOS et Linux.

### Bundle frontend

Le build sépare xterm et l'API Tauri du code d'application. Tailles minifiées :

- application : 236,38 kB, 74,07 kB gzip ;
- xterm : 332,16 kB, 83,42 kB gzip ;
- API Tauri : 1,14 kB, 0,55 kB gzip ;
- CSS total : 14,59 kB, 3,99 kB gzip.

Le build ne déclenche plus l'avertissement de chunk supérieur à 500 kB. Les
mises à jour de trafic identiques ne provoquent plus de rendu React, et les
panneaux terminal inactifs sont mémoïsés.

## Matrice multiplateforme

| Système | Build | Tests Rust | Probe PTY | Contrôle manuel | État |
| --- | --- | --- | --- | --- | --- |
| Windows 11 | réussi | réussi | réussi | réussi | validé localement |
| macOS | non exécuté | non exécuté | non exécuté | non exécuté | en attente |
| Linux | non exécuté | non exécuté | non exécuté | non exécuté | en attente |

Pour chaque système restant, exécuter la validation de
[desktop.md](desktop.md), le probe ci-dessus, puis relever version du système,
matériel, trois démarrages, mémoire du processus complet et captures à
1280×720 et 560×400. Une CI qui échoue avant sa première étape n'est pas une
preuve : T124 reste donc ouvert jusqu'à obtention des deux lignes manquantes.
