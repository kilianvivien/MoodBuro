<p align="center">
  <img src="assets/app-icon.png" width="112" alt="Icône MoodBuro" />
</p>
<h1 align="center">MoodBuro</h1>
<p align="center"><strong>Votre Moodle, à sa place sur votre Mac.</strong><br/>Une application de bureau légère pour retrouver le Moodle de votre organisation.</p>
<p align="center">
  <a href="https://github.com/kilianvivien/MoodBuro/actions/workflows/ci.yml"><img src="https://github.com/kilianvivien/MoodBuro/actions/workflows/ci.yml/badge.svg?branch=main" alt="CI" /></a>
  <img src="https://img.shields.io/badge/version-0.1.0-blue" alt="Version 0.1.0" />
  <img src="https://img.shields.io/badge/Tauri-2-24c8db" alt="Tauri 2" />
  <img src="https://img.shields.io/badge/interface-FR%20%2F%20EN-007aff" alt="Interface français et anglais" />
  <a href="LICENSE"><img src="https://img.shields.io/badge/licence-MIT-green" alt="Licence MIT" /></a>
</p>
<p align="center">Français · <a href="README.en.md">English</a></p>

MoodBuro affiche **le vrai site Moodle** dans une application macOS, avec vos organisations, vos cours et vos outils à portée de main. Vous renseignez l’adresse Moodle de votre organisation et vous vous connectez sur sa propre page.

## Télécharger et installer

**[Télécharger MoodBuro 0.1.0 pour macOS](https://github.com/kilianvivien/MoodBuro/releases/latest)**

Le DMG universel contient les versions **Apple Silicon et Intel**, avec macOS 12 comme version minimale configurée. Téléchargez le DMG et `SHA256SUMS.txt`, vérifiez la somme de contrôle, puis glissez MoodBuro dans Applications. Pour une mise à jour, quittez d’abord l’application.

La version 0.1.0 utilise une **signature ad hoc et n’est pas notariée**. macOS peut demander d’autoriser sa première ouverture dans Réglages Système → Confidentialité et sécurité. Les mises à jour s’installent manuellement.

## Vos cours, dans une interface pensée pour le Mac

- **Plusieurs organisations Moodle**, chacune avec son accès, son abréviation personnalisable et, sur macOS 14 et versions ultérieures, son profil WebKit persistant distinct.
- **Une interface discrète**, avec de vrais contrôles de fenêtre, des menus natifs, une barre d’outils compacte et une fenêtre qui retrouve sa taille et sa position.
- **Français et anglais**, langue du système ou choix explicite, apparence automatique, claire ou sombre et prise en compte des préférences de contraste et de mouvement.
- **Ouverture rapide avec ⌘K**, pour retrouver cours, activités, organisations et vues locales au-dessus de la page en cours.

| Fonction | Ce que vous pouvez faire |
| --- | --- |
| Signets | Enregistrer une page Moodle avec ⌘D, personnaliser son nom et choisir parmi huit icônes. Cinq signets sont visibles dans la barre latérale ; les suivants sont accessibles dans un menu natif. |
| Notifications | Consulter la boîte de réception, gérer les notifications lues et retrouver les compteurs dans la barre latérale, le Dock et la barre des menus. |
| Cours et agenda | Retrouver les cours, leurs activités et les échéances, épingler des cours et exporter l’agenda au format ICS. |
| Rappels | Activer les rappels locaux par organisation ou choisir un rappel pour une échéance. Ils sont désactivés par défaut. |
| Téléchargements | Suivre les transferts, retrouver l’historique, ouvrir les fichiers ou les afficher dans le Finder, avec un classement par organisation et par cours lorsque celui-ci est identifié. |
| Fichiers hors connexion | Sélectionner des fichiers de cours à conserver sur le Mac et les actualiser manuellement, en préservant les modifications locales et les versions précédentes. |
| Lecture de documents | Consulter les PDF et documents Office pris en charge dans des fenêtres de lecture natives. |
| Assistant IA facultatif | Utiliser Albert API ou Mistral AI avec votre propre clé, joindre explicitement un contexte Moodle, consulter les sources et confirmer les actions proposées. |

L’assistant peut notamment retrouver des cours, activités, notifications, fichiers et échéances. Les actions qui modifient des données ou enregistrent un fichier demandent une confirmation. Les conversations restent en mémoire jusqu’à la fermeture de l’application.

## Raccourcis utiles

| Action | Raccourci |
| --- | --- |
| Ouverture rapide | ⌘K |
| Ajouter la page aux signets | ⌘D |
| Assistant IA | ⇧⌘A |
| Notifications · Cours · Agenda | ⌘1 · ⌘2 · ⌘3 |
| Réglages | ⌘, |
| Quitter | ⌘Q |

Fermer la fenêtre la masque et laisse le suivi des notifications et des rappels actif. **Quitter** arrête l’application. Si Moodle limite temporairement la connexion automatique après un redémarrage, la barre d’outils explique l’attente et indique l’heure du prochain essai.

## Identifiants et données locales

MoodBuro ne dessine pas de formulaire de mot de passe Moodle. Les jetons et clés d’API restent côté Rust et dans le trousseau du système ; ils ne sont pas stockés dans SQLite ni transmis à l’interface locale. Sur macOS, ils partagent un élément du trousseau et une lecture au démarrage. La première migration peut encore demander une autorisation pour chaque ancien élément.

Les pages Moodle et les fenêtres de connexion n’ont aucun accès aux commandes privilégiées de l’application. Les téléchargements sont limités à l’origine HTTPS autorisée. Les signets sont locaux et liés à leur compte ; le retrait ou le remplacement du compte les supprime.

Le retrait d’une organisation efface ses identifiants et métadonnées et demande la suppression de son profil isolé. Les fichiers téléchargés sont conservés. L’effacement WebKit est asynchrone ; certaines données et sessions SSO partagées peuvent subsister sur les anciennes versions de macOS.

## Développer et vérifier

Installez Node.js 22+, pnpm 9.15.9, Rust et les outils Xcode. Le compilateur Rust est fixé dans `rust-toolchain.toml`.

```sh
git clone https://github.com/kilianvivien/MoodBuro.git
cd MoodBuro
pnpm install --frozen-lockfile
pnpm tauri dev
```

`pnpm dev` lance seulement l’interface web. La connexion Moodle et le stockage sécurisé nécessitent Tauri. Le mode de démonstration `http://127.0.0.1:1420/?mac&demo` utilise des données fictives en mémoire, sans connexion réelle.

```sh
pnpm lint
pnpm format:check
pnpm test
pnpm build
cargo fmt --all --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
python3 scripts/check-security.py
```

La version 0.1.0 a passé **109 tests d’interface et 112 tests Rust**, ainsi que les vérifications de format, de compilation, Clippy et de permissions IPC. Les tests automatisés et les contrats HTTP simulés ne remplacent pas la qualification sur des comptes et environnements réels.

## Architecture

```text
apps/shell/          React · TypeScript · Vite · Zustand · interface FR/EN
src-tauri/           Fenêtres natives, menus, commandes et permissions
crates/md-moodle/    Protocole Moodle, HTTP et politiques de navigation
crates/md-secrets/   Trousseau du système
crates/md-store/     Métadonnées SQLite et préférences
crates/md-assistant/ Fournisseurs IA, conversations et outils contrôlés
crates/md-notifier/  Décisions de suivi des notifications
crates/md-os/        Adaptateurs système et opérations sur les fichiers
docs/               Plans, décisions, guides et état d’implémentation
```

## Compatibilité et suite du projet

La distribution actuelle cible macOS. La qualification complète des versions de macOS, de l’exécution sur Intel, des fournisseurs SSO, des notifications natives et de l’accessibilité reste ouverte, de même que la signature Developer ID, la notarisation et les mises à jour automatiques. Windows et Linux sont des cibles ultérieures.

Le lecteur PDF utilise un rappel privé de WebKit pour l’enregistrement ; sa compatibilité doit être vérifiée sur les versions de macOS prises en charge. MoodBuro est générique : aucune organisation n’est préconfigurée. Le projet est indépendant de Moodle HQ.


## Licence

[MIT](LICENSE) — © 2026 Kilian Vivien.

## Mises à jour bêta

Le workflow macOS prépare des archives universelles signées pour les tags `v0.*-beta.*`, avec un flux bêta distinct sur GitHub Releases. La première version compatible doit être installée manuellement. Réglages → Général permet la recherche automatique au lancement puis toutes les six heures, sa désactivation, la consultation des notes, le suivi du téléchargement, l’installation et le relancement explicites. La version 0.1.0 reste à mettre à jour manuellement. La qualification des mises à jour sur Intel et Apple Silicon reste ouverte. Voir les consignes de publication dans le [README anglais](README.en.md#beta-update-releases-maintainers).
