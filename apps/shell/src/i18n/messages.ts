import { formatNumber, plural, type Locale } from "./locale";

/**
 * Every string the shell shows or announces. English is the reference; the
 * French catalogue must have the same shape, which TypeScript enforces.
 * French follows macOS conventions (Réglages, Terminé, Précédent/Suivant).
 */
const en = {
  sidebar: {
    label: "Moodle sites",
    unread: (count: number) => `${formatNumber("en", count)} unread`,
    connection: {
      checking: "Checking for updates",
      online: "",
      offline: "Offline",
      reauth: "Reconnect in Settings",
      error: "Could not check for updates",
    },
    add: "Add organisation",
    settings: "Settings",
  },
  toolbar: {
    home: "Moodle Home",
    backToMoodle: "Back to Moodle",
    popup: "Moodle window",
    downloads: "Downloads",
    navigation: "Navigation",
    back: "Back",
    forward: "Forward",
    reload: "Reload page",
    openInBrowser: "Open page in browser",
    dismiss: "Dismiss",
    dismissLabel: "Dismiss message",
    settings: "Settings",
    addOrganisation: "Add Organisation",
    done: "Done",
    cancel: "Cancel",
  },
  downloads: {
    openLabel: (name: string) => `Open ${name}`,
    revealLabel: (name: string) => `Show ${name} in Finder`,
    title: "Recent downloads",
    empty: "No downloads yet.",
    loading: "Loading downloads…",
    location:
      "Files are saved in ~/Moodle, in a folder for each organisation. Removing an organisation keeps its downloaded files.",
    open: "Open",
    reveal: "Show in Finder",
    cancel: "Cancel",
    confirm: "Open File",
    confirmTitle: (name: string) => `Open ${name}?`,
    confirmBody:
      "This file type may run software or contain active content. Open it only if you trust its source.",
    states: {
      downloading: "Downloading…",
      complete: "Complete",
      failed: "Failed — download again from Moodle",
    },
  },
  onboarding: {
    welcome: "Welcome to MoodBuro",
    addTitle: "Add an organisation",
    lead: "Enter the address of your organisation’s Moodle. You’ll sign in on its own page, exactly as you do in a browser.",
    fieldLabel: "Moodle address",
    placeholder: "moodle.example.edu",
    help: "The address you use to reach Moodle, such as moodle.example.edu. HTTPS only.",
    findSite: "Finding your Moodle site",
    signIn: "Signing in with your organisation",
    stepDone: " — done",
    stepActive: " — in progress",
    continueInWindow: "Continue in the sign-in window. Close it to cancel.",
    blocked: "Finish the open sign-in window first.",
    submit: "Continue",
    privacy:
      "MoodBuro never sees your password. Its access token is kept in your Mac’s Keychain.",
  },
  settings: {
    previewBanner:
      "Browser preview: appearance and language changes last until reload, and accounts can only be managed in the desktop app.",
    general: "General",
    shortLabel: "Short label",
    shortLabelHint:
      "Up to 4 letters or numbers; leave blank for automatic initials.",
    saveLabel: "Save",
    labelSaved: "Saved",
    theme: "Theme",
    themeHint: "Automatic matches your Mac’s appearance setting.",
    appearance: { system: "Automatic", light: "Light", dark: "Dark" },
    language: "Language",
    languageHint: "Automatic follows your Mac’s preferred language.",
    languages: { system: "Automatic", fr: "Français", en: "English" },
    organisations: "Organisations",
    siteCount: (count: number) =>
      `${formatNumber("en", count)} ${plural("en", count) === "one" ? "site" : "sites"}`,
    empty: "No organisations connected yet.",
    add: "Add Organisation…",
    footnote:
      "Sign in again if a site stops loading your courses. Removing a site disconnects it from this Mac. Shared sign-in sessions on older macOS versions may remain.",
    open: "Open",
    signInAgain: "Sign In Again",
    signInAgainLabel: (name: string) => `Sign in again to ${name}`,
    remove: "Remove…",
    removeLabel: (name: string) => `Remove ${name}`,
    connecting: "Connecting to the site…",
    continueInWindow: "Continue in the sign-in window. Close it to cancel.",
    removeTitle: (name: string) => `Remove ${name}?`,
    removeBody:
      "This organisation’s saved account and Keychain credentials will be removed from this Mac. Its private browser profile will be cleared where supported. On older macOS versions, shared sign-in sessions may remain. Your Moodle account and courses are not affected.",
    cancel: "Cancel",
    confirmRemove: "Remove",
    removed: (name: string) => `${name} was removed from this Mac.`,
    removedUnnamed: "The site was removed from this Mac.",
    reauthenticated: (name: string) => `Signed in to ${name} again.`,
    reauthenticatedUnnamed: "Signed in to the site again.",
  },
  errors: {
    /** Shown in French only when a backend message has no translation. */
    general: "Something went wrong.",
    add: "Could not connect this organisation.",
    signIn: "Could not sign in again.",
    settings: "Could not save this setting.",
    remove: "Could not remove this organisation.",
    navigation: "Could not complete this action.",
    detail: "Details:",
  },
};

export type Messages = typeof en;

const fr: Messages = {
  sidebar: {
    unread: (count: number) =>
      `${formatNumber("fr", count)} non lu${count > 1 ? "s" : ""}`,
    connection: {
      checking: "Vérification en cours",
      online: "",
      offline: "Hors ligne",
      reauth: "Reconnecter dans Réglages",
      error: "Impossible de vérifier les mises à jour",
    },
    label: "Sites Moodle",
    add: "Ajouter une organisation",
    settings: "Réglages",
  },
  toolbar: {
    home: "Accueil Moodle",
    backToMoodle: "Retour à Moodle",
    popup: "Fenêtre Moodle",
    downloads: "Téléchargements",
    navigation: "Navigation",
    back: "Précédent",
    forward: "Suivant",
    reload: "Recharger la page",
    openInBrowser: "Ouvrir la page dans le navigateur",
    dismiss: "Fermer",
    dismissLabel: "Fermer le message",
    settings: "Réglages",
    addOrganisation: "Ajouter une organisation",
    done: "Terminé",
    cancel: "Annuler",
  },
  downloads: {
    openLabel: (name: string) => `Ouvrir ${name}`,
    revealLabel: (name: string) => `Afficher ${name} dans le Finder`,
    title: "Téléchargements récents",
    empty: "Aucun téléchargement pour le moment.",
    loading: "Chargement des téléchargements…",
    location:
      "Les fichiers sont enregistrés dans ~/Moodle, dans un dossier par organisation. Retirer une organisation conserve ses fichiers téléchargés.",
    open: "Ouvrir",
    reveal: "Afficher dans le Finder",
    cancel: "Annuler",
    confirm: "Ouvrir le fichier",
    confirmTitle: (name: string) => `Ouvrir ${name}\u00a0?`,
    confirmBody:
      "Ce type de fichier peut exécuter un logiciel ou contenir du contenu actif. Ouvrez-le uniquement si sa source est fiable.",
    states: {
      downloading: "Téléchargement…",
      complete: "Terminé",
      failed: "Échec — téléchargez à nouveau depuis Moodle",
    },
  },
  onboarding: {
    welcome: "Bienvenue dans MoodBuro",
    addTitle: "Ajouter une organisation",
    lead: "Saisissez l’adresse du Moodle de votre organisation. Vous vous connecterez sur sa propre page, exactement comme dans un navigateur.",
    fieldLabel: "Adresse Moodle",
    placeholder: "moodle.exemple.fr",
    help: "L’adresse que vous utilisez pour accéder à Moodle, par exemple moodle.exemple.fr. HTTPS uniquement.",
    findSite: "Recherche de votre site Moodle",
    signIn: "Connexion avec votre organisation",
    stepDone: " — terminé",
    stepActive: " — en cours",
    continueInWindow:
      "Poursuivez dans la fenêtre de connexion. Fermez-la pour annuler.",
    blocked: "Terminez d’abord la connexion dans la fenêtre ouverte.",
    submit: "Continuer",
    privacy:
      "MoodBuro ne voit jamais votre mot de passe. Son jeton d’accès est conservé dans le trousseau de votre Mac.",
  },
  settings: {
    previewBanner:
      "Aperçu dans le navigateur\u00a0: les changements d’apparence et de langue durent jusqu’au rechargement, et les comptes ne peuvent être gérés que dans l’app de bureau.",
    general: "Général",
    shortLabel: "Abréviation",
    shortLabelHint:
      "4 lettres ou chiffres maximum ; laisser vide pour les initiales automatiques.",
    saveLabel: "Enregistrer",
    labelSaved: "Enregistré",
    theme: "Thème",
    themeHint: "Automatique suit le réglage d’apparence de votre Mac.",
    appearance: { system: "Automatique", light: "Clair", dark: "Sombre" },
    language: "Langue",
    languageHint: "Automatique suit la langue préférée de votre Mac.",
    languages: { system: "Automatique", fr: "Français", en: "English" },
    organisations: "Organisations",
    siteCount: (count: number) =>
      `${formatNumber("fr", count)} ${plural("fr", count) === "one" ? "site" : "sites"}`,
    empty: "Aucune organisation connectée pour l’instant.",
    add: "Ajouter une organisation…",
    footnote:
      "Reconnectez-vous si un site ne charge plus vos cours. Retirer un site le déconnecte de ce Mac. Sur les versions plus anciennes de macOS, des sessions de connexion partagées peuvent subsister.",
    open: "Ouvrir",
    signInAgain: "Se reconnecter",
    signInAgainLabel: (name: string) => `Se reconnecter à ${name}`,
    remove: "Retirer…",
    removeLabel: (name: string) => `Retirer ${name}`,
    connecting: "Connexion au site…",
    continueInWindow:
      "Poursuivez dans la fenêtre de connexion. Fermez-la pour annuler.",
    removeTitle: (name: string) => `Retirer ${name}\u00a0?`,
    removeBody:
      "Le compte enregistré de cette organisation et ses identifiants du trousseau seront supprimés de ce Mac. Son profil de navigation privé sera effacé lorsque c’est possible. Sur les versions plus anciennes de macOS, des sessions de connexion partagées peuvent subsister. Votre compte Moodle et vos cours ne sont pas affectés.",
    cancel: "Annuler",
    confirmRemove: "Retirer",
    removed: (name: string) => `${name} a été retiré de ce Mac.`,
    removedUnnamed: "Le site a été retiré de ce Mac.",
    reauthenticated: (name: string) => `Connexion à ${name} rétablie.`,
    reauthenticatedUnnamed: "Connexion au site rétablie.",
  },
  errors: {
    general: "Une erreur inattendue s’est produite.",
    add: "Impossible de connecter cette organisation.",
    signIn: "Impossible de vous reconnecter.",
    settings: "Impossible d’enregistrer ce réglage.",
    remove: "Impossible de retirer cette organisation.",
    navigation: "Impossible d’effectuer cette action.",
    detail: "Détail\u00a0:",
  },
};

export const catalogues: Record<Locale, Messages> = { en, fr };
