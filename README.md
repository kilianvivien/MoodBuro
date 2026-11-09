<p align="center">
  <img src="assets/icon.svg" width="112" alt="Icône MoodBuro" />
</p>
<h1 align="center">MoodBuro</h1>
<p align="center"><strong>Votre Moodle, à sa place sur votre Mac.</strong><br/>Une application de bureau légère pour retrouver le Moodle de votre organisation.</p>
<p align="center">
  <a href="https://github.com/kilianvivien/MoodBuro/actions/workflows/ci.yml"><img src="https://github.com/kilianvivien/MoodBuro/actions/workflows/ci.yml/badge.svg?branch=main" alt="CI" /></a>
  <img src="https://img.shields.io/badge/statut-alpha_en_d%C3%A9veloppement-blue" alt="Alpha en développement" />
  <img src="https://img.shields.io/badge/Tauri-2-24c8db" alt="Tauri 2" />
  <img src="https://img.shields.io/badge/interface-FR%20%2F%20EN-007aff" alt="Interface français et anglais" />
  <a href="LICENSE"><img src="https://img.shields.io/badge/licence-MIT-green" alt="Licence MIT" /></a>
</p>
<p align="center">Français · <a href="README.en.md">English</a></p>

MoodBuro affiche **le vrai site Moodle** dans une application macOS, avec une interface discrète autour de vos cours. Vous renseignez l’adresse Moodle de votre organisation, puis vous vous connectez sur sa propre page. L’application ne dessine jamais de formulaire de mot de passe.

> **État du projet :** premières phases de l’Alpha. L’application se compile et dispose de tests automatisés, mais la connexion SSO réelle et la matrice de compatibilité ne sont pas encore validées. Aucun installateur signé ou notarié n’est publié pour le moment.

## Une interface pensée pour macOS

- **Une coque discrète :** barre latérale, barre d’outils compacte, icônes cohérentes et vrais contrôles de fenêtre macOS.
- **Barre de titre intégrée :** titre centré, surface continue et lisible, zone de déplacement et véritables boutons macOS.
- **Français et anglais dès l’accueil :** langue du système par défaut, choix explicite dans Réglages, modes automatique/clair/sombre et prise en compte des préférences d’accessibilité.
- **Plusieurs organisations :** ajout d’une URL Moodle, changement de site et retour rapide à vos cours.
- **Des réglages utiles :** apparence, langue, abréviation personnalisable pour chaque organisation, reconnexion et retrait avec confirmation.

## Ce qui fonctionne déjà

| Domaine | Implémentation actuelle |
| --- | --- |
| Accueil | Saisie de l’URL de l’organisation, découverte de la configuration mobile et suivi de la connexion |
| Authentification | Fenêtre dédiée, vérification du retour signé et validation du jeton auprès du serveur |
| Sessions | Restauration par l’API d’autoconnexion, réutilisation des vues déjà ouvertes et reconnexion |
| Identifiants | Stockage des jetons dans le trousseau du système ; aucune copie dans SQLite ou le navigateur local |
| Navigation | Précédent, suivant, rechargement, ouverture dans le navigateur et menus/raccourcis natifs |
| Profils | Stockage WebKit distinct par organisation sur macOS 14 et versions ultérieures |
| Qualité | Tests Rust et React, contrats HTTP simulés, vérifications des permissions et CI Linux/macOS |

Le premier site de validation est celui de l’INSP. Il n’est **pas préconfiguré** : MoodBuro reste une application générique pour les organisations disposant de Moodle et de ses services mobiles.

## Démarrer en développement

Sur Mac, installez Node.js 22+, pnpm 9.15.9, Rust et les outils de ligne de commande Xcode. Le dépôt fixe la version du compilateur Rust dans `rust-toolchain.toml`.

```sh
git clone https://github.com/kilianvivien/MoodBuro.git
cd MoodBuro
pnpm install --frozen-lockfile
pnpm tauri dev
```

Pour travailler uniquement sur l’interface : `pnpm dev`. La connexion Moodle et le stockage sécurisé nécessitent l’application Tauri ; le mode navigateur ne simule pas une connexion réussie.

Pour produire une application macOS locale de développement :

```sh
pnpm tauri build --debug --bundles app
```

Cette commande ne produit pas une version de distribution signée et notariée.

## Architecture et sécurité

```text
apps/shell/       React · TypeScript · Vite · Zustand · interface FR/EN
src-tauri/        Fenêtres natives, menus, commandes et permissions
crates/md-moodle/ Protocole Moodle, HTTP et politiques de navigation
crates/md-secrets/ Trousseau du système
crates/md-store/  Métadonnées SQLite et préférences
crates/md-notifier/ et md-os/  Points d’extension pour les étapes suivantes
docs/             PRD, plan Alpha, décisions et état d’implémentation
```

Seule l’interface locale possède des permissions IPC. Les pages Moodle et les fenêtres de connexion n’ont aucun accès aux commandes privilégiées. Les jetons restent côté Rust et dans le trousseau ; les erreurs HTTP sont filtrées et les URL d’autoconnexion ne sont pas transmises à l’interface.

Le retrait d’un site supprime ses identifiants et ses métadonnées, et demande l’effacement de son profil isolé. L’effacement WebKit est asynchrone. Sur les anciennes versions de macOS utilisant le stockage partagé, certaines données de navigateur et sessions SSO partagées peuvent subsister : c’est un point de validation encore ouvert.

## Vérifier une modification

```sh
pnpm build
pnpm lint
pnpm test
pnpm format:check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
python3 scripts/check-security.py
```

Les contrats HTTP utilisent des réponses simulées. Ils complètent les essais réels ; ils ne prouvent pas la compatibilité avec toutes les versions de Moodle ou tous les fournisseurs SSO.

## Prochaines étapes

- Valider la connexion INSP et les environnements Moodle 4.5/5.3 avec SAML et OIDC.
- Vérifier les profils, le trousseau et l’isolation IPC dans l’application installée.
- Ajouter notifications, badge et présence dans la barre des menus.
- Implémenter les liens profonds et la capture des téléchargements.
- Préparer les distributions universelles macOS, puis la signature et la notarisation.

Les téléchargements sont actuellement bloqués dans les vues Moodle. Fermer la fenêtre quitte l’application tant que le fonctionnement en arrière-plan n’est pas implémenté. Windows et Linux sont des cibles ultérieures, pas des plateformes validées de cette Alpha.

## Documentation

[PRD](docs/prd.md) · [Plan Alpha](docs/alpha-plan.md) · [État de l’implémentation](docs/implementation-status.md) · [Décisions d’architecture](docs/adr/0001-alpha-foundation.md) · [Interface](docs/ui-design.md)

Le nom « Moodle Desk » présent dans les documents initiaux est le nom de travail historique. Le code utilise MoodBuro. Le projet est indépendant de Moodle HQ.

## Licence

[MIT](LICENSE) — © 2026 Kilian Vivien.
