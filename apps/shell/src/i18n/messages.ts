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
    inbox: "Inbox",
    courses: "Courses",
    agenda: "Agenda",
    views: "Views",
    organisations: "Organisations",
  },
  common: {
    refresh: "Refresh",
    refreshing: "Refreshing…",
    updatedNow: "Updated just now",
    updated: (when: string) => `Updated ${when}`,
    never: "Not refreshed yet",
    allSites: "All organisations",
    siteFilter: "Organisation",
    loading: "Loading…",
    refreshFailed: (site: string) =>
      `${site} could not be refreshed. The saved list is shown.`,
    previewOnly:
      "Browser preview with sample data. Connect an organisation in the desktop app to see your own.",
  },
  inbox: {
    title: "Notifications",
    filter: "Show",
    all: "All",
    unread: "Unread",
    unreadCount: (count: number) => `${formatNumber("en", count)} unread`,
    empty: "No notifications yet",
    emptyHint:
      "New Moodle notifications appear here while MoodBuro is running.",
    caughtUp: "You’re all caught up",
    caughtUpHint: "There are no unread notifications.",
    open: "Open",
    openLabel: (subject: string) => `Open in Moodle: ${subject}`,
    markRead: "Mark as Read",
    markReadLabel: (subject: string) => `Mark as read: ${subject}`,
    markAll: "Mark All as Read",
    unreadState: "Unread",
    messages: (count: number, site: string) =>
      `${formatNumber("en", count)} unread ${plural("en", count) === "one" ? "conversation" : "conversations"} in ${site}`,
    messagesHint: "Conversations are read in Moodle.",
    openMessages: "Open Messages",
    footnote:
      "MoodBuro keeps the latest 100 notification subjects per organisation on this Mac. Opening one shows that organisation’s notifications page in Moodle.",
  },
  offline: {
    files: "Files…",
    filesLabel: (name: string) => `Offline files for ${name}`,
    title: "Course files",
    back: "Back to Courses",
    openCourse: "Open Course in Moodle",
    hint: "Save selected plain files from Moodle resources and folders. Saved copies open without a connection. Loading and refreshing need a connection and permission to download files. Refresh is manual and one-way: changed files are saved alongside existing copies, local edits are kept, and Moodle deletions never delete local files. Account removal keeps files in the course’s Finder folder but clears this list. This is not a complete offline course.",
    savedTitle: "Saved on This Mac",
    availableTitle: "Available from Moodle",
    load: "Load File List",
    emptySaved:
      "No saved copies for this course yet. Load the file list to choose files.",
    noResources:
      "No supported files were returned. Open the course in Moodle to find other resources.",
    missing:
      "This copy was moved or is unavailable. Refresh to save a new copy.",
    refresh: "Refresh Copy",
    refreshLabel: (name: string) => `Refresh offline copy of ${name}`,
    unknownSize: "Size unknown",
    bytes: (bytes: string) => `${bytes} bytes`,
    tooLarge: "Exceeds the 128 MB limit. Download this file from Moodle.",
    limit:
      "Up to 50 files per selection, 128 MB each. One transfer at a time; at least 144 MB free space required. The latest 2,000 saved copies are shown. Transfers stop when you quit.",
    saveSelected: (count: number) => `Save Selected (${count})`,
    working: "Working on course files…",
    loading: "Loading saved copies…",
    progress: (bytes: string) =>
      `${bytes} bytes received for the current file.`,
    results: "Transfer results",
    file: "Selected file",
    unchanged: "Unchanged — existing copy kept.",
    saved: "Saved a new copy.",
    skipped: (count: number) =>
      `${count} unsupported file entries were omitted. Only plain files in Moodle resources and folders are supported.`,
  },
  courses: {
    refreshActivities: "Refresh Activities",
    title: "Courses",
    searchLabel: "Search courses",
    searchPlaceholder: "Search courses",
    pinned: "Pinned",
    pin: "Pin",
    unpin: "Unpin",
    pinLabel: (name: string) => `Pin ${name}`,
    unpinLabel: (name: string) => `Unpin ${name}`,
    open: "Open",
    openLabel: (name: string) => `Open ${name} in Moodle`,
    count: (count: number) =>
      `${formatNumber("en", count)} ${plural("en", count) === "one" ? "course" : "courses"}`,
    empty: "No courses yet",
    emptyHint: "Courses you are enrolled in appear here after a refresh.",
    noMatch: (query: string) => `No course matches “${query}”`,
    noMatchHint: "Search looks at course and activity names saved on this Mac.",
    footnote:
      "Pins are saved on this Mac only and do not change your Moodle favourites. This list stays available offline; opening a course needs a connection.",
  },
  reminders: {
    title: "Local deadline reminders",
    day: "24 hours before",
    hour: "1 hour before",
    label: (site: string, offset: string) => `${site}: ${offset}`,
    hint: "Off by default. Options apply to each organisation’s upcoming deadlines, independently of agenda filters. MoodBuro periodically refreshes enabled calendars and uses snapshots no older than 15 minutes. Events reported as non-actionable or with no remaining items are skipped; other completion states may be unknown.",
    limits:
      "Reminders work while MoodBuro is running, including with its window closed. Startup, wake and clock changes skip missed reminders. Nothing is promised while quit or offline. macOS notification permissions and Focus can suppress delivery; check System Settings → Notifications. This app cannot verify that a notification was displayed.",
    never:
      "No successful agenda refresh yet. Enable a reminder to request one.",
    updated: (date: string) => `Agenda refreshed ${date}`,
    stale: "Agenda data is stale. Reminders wait for a successful refresh.",
    deliveryFailed:
      "A notification attempt failed. Check macOS notification settings. Failed or interrupted attempts are not replayed.",
    saving: "Saving reminder preferences…",
  },
  agenda: {
    title: "Upcoming",
    export: "Export Calendar…",
    exporting: "Exporting…",
    from: "From",
    through: "Through",
    invalidRange:
      "Choose a valid date range with the end on or after the start.",
    exportHint:
      "Export saves the filtered, cached agenda to Downloads and shows the file in Finder. Import the .ics file into Calendar yourself. This snapshot does not update automatically; repeated imports may create duplicates.",
    exported: (count: number, filename: string) =>
      `Saved ${count} events to Downloads: ${filename}.`,
    revealFailed:
      "Finder could not show the file. Open your Downloads folder to find it.",
    count: (count: number) =>
      `${formatNumber("en", count)} ${plural("en", count) === "one" ? "event" : "events"}`,
    empty: "Nothing coming up",
    emptyHint: "Deadlines and events from your Moodle calendars appear here.",
    kinds: {
      due: "Due",
      open: "Opens",
      user: "Personal",
      site: "Site",
      course: "Course",
      other: "Event",
    },
    until: (time: string) => `until ${time}`,
    open: "Open",
    openLabel: (name: string) => `Open ${name} in the Moodle calendar`,
    timeZone: (zone: string) =>
      `Times are shown in your Mac’s time zone (${zone}).`,
    footnote:
      "This agenda is read-only and shows what Moodle reported at the last refresh. Moodle remains the reference for submissions and completion.",
  },
  quickOpen: {
    label: "Quick Open",
    placeholder: "Open a course, activity, organisation or view",
    cacheDate: (date: string) =>
      `Saved metadata · oldest refresh: ${date}. Moodle pages require a connection.`,
    cacheEmpty:
      "No saved metadata yet. Refresh courses and activities in Courses.",
    results: "Results",
    courses: "Courses",
    organisations: "Organisations",
    views: "Views",
    pinned: "Pinned",
    empty: "No results",
    emptyHint: "Search looks at course and activity names saved on this Mac.",
    hint: "↑ ↓ to choose · Return to open · Esc to close",
  },
  toolbar: {
    inbox: "Inbox",
    courses: "Courses",
    agenda: "Agenda",
    quickOpen: "Quick Open (⌘L)",
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
    preview: "Preview",
    previewLabel: (name: string) => `Preview ${name}`,
    previewHint:
      "PDF, Word, Excel and PowerPoint files use the same native macOS reader. Use Open to edit in your default app. Previews require the desktop app.",
    courseFilter: "Course",
    allCourses: "All courses",
    unknownCourse: "Unknown course",
    refreshFolders: "Refresh Course Folders",
    returnToMoodle: "Return to Moodle",
    returnHint:
      "For a failed download, return to Moodle and start it again there. This does not restart the transfer.",
    foldersRefreshed:
      "Course folders refreshed. New downloads will use known course folders; existing files stay in place.",
    openLabel: (name: string) => `Open ${name}`,
    revealLabel: (name: string) => `Show ${name} in Finder`,
    title: "Downloads",
    searchLabel: "Search downloads",
    searchPlaceholder: "Search by file name",
    stateFilter: "State",
    allStates: "All",
    filters: {
      downloading: "In progress",
      complete: "Complete",
      failed: "Failed",
    },
    shown: (shown: number, total: number) =>
      `${formatNumber("en", shown)} of ${formatNumber("en", total)}`,
    showMore: "Show More",
    noMatch: "No downloads match",
    noMatchHint: "Try another name, organisation or state.",
    emptyHint: "Files you download from Moodle are listed here.",
    copyPath: "Copy Path",
    copyPathLabel: (name: string) => `Copy the path of ${name}`,
    copied: "Path copied to the clipboard.",
    more: (name: string) => `More actions for ${name}`,
    clear: "Clear History…",
    clearTitle: "Clear download history?",
    clearTitleFor: (site: string) => `Clear download history for ${site}?`,
    clearBody:
      "Only this list is cleared. Downloaded files stay in ~/Moodle, and transfers in progress stay listed.",
    confirmClear: "Clear History",
    cleared: "History cleared. Your files were kept.",
    empty: "No downloads yet",
    loading: "Loading downloads…",
    location:
      "Files are saved in ~/Moodle, by organisation and known course. Refresh Course Folders to index Moodle resources. Choose an organisation to filter by course. Unmatched files stay in Downloads. Removing an organisation keeps its files.",
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
      "Browser preview: settings changes last until reload, and accounts can only be managed in the desktop app.",
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
    globalShortcut: "Quick Open from any app",
    globalShortcutHint:
      "⌘⇧M shows MoodBuro and opens Quick Open while the app is running.",
    shortcutFootnote:
      "macOS does not always report a shortcut conflict: if another app already uses ⌘⇧M, that app may keep it.",
    shortcutOptions: { off: "Off", on: "On" },
    shortcutUnavailable:
      "⌘⇧M could not be registered when MoodBuro started. Turn it off and on again to retry.",
    files: "Files",
    pdf: "PDF documents",
    pdfHint: "Opened directly by a Moodle page.",
    images: "Images",
    imagesHint: "Opened directly by a Moodle page.",
    handling: { inline: "View in Moodle", save: "Save", open: "Save and Open" },
    filesFootnote:
      "Saved files go to the course folder, or to Downloads when the course is unknown. Save and Open then opens the file in its default app. Files embedded in a Moodle page stay in the page. To save the file you are viewing, choose File › Save to Downloads (⌘S).",
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
    moveUp: (name: string) => `Move ${name} Up`,
    moveDown: (name: string) => `Move ${name} Down`,
    orderHint:
      "Use the arrows to change organisation order in the sidebar and menu bar. Order is saved on this Mac.",
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
    inbox: "Boîte de réception",
    courses: "Cours",
    agenda: "Agenda",
    views: "Vues",
    organisations: "Organisations",
  },
  common: {
    refresh: "Actualiser",
    refreshing: "Actualisation…",
    updatedNow: "Actualisé à l’instant",
    updated: (when: string) => `Actualisé ${when}`,
    never: "Pas encore actualisé",
    allSites: "Toutes les organisations",
    siteFilter: "Organisation",
    loading: "Chargement…",
    refreshFailed: (site: string) =>
      `Impossible d’actualiser ${site}. La liste enregistrée est affichée.`,
    previewOnly:
      "Aperçu dans le navigateur avec des données d’exemple. Connectez une organisation dans l’app de bureau pour voir les vôtres.",
  },
  inbox: {
    title: "Notifications",
    filter: "Afficher",
    all: "Toutes",
    unread: "Non lues",
    unreadCount: (count: number) =>
      `${formatNumber("fr", count)} non ${count > 1 ? "lues" : "lue"}`,
    empty: "Aucune notification pour le moment",
    emptyHint:
      "Les nouvelles notifications Moodle apparaissent ici tant que MoodBuro est ouvert.",
    caughtUp: "Vous êtes à jour",
    caughtUpHint: "Il n’y a aucune notification non lue.",
    open: "Ouvrir",
    openLabel: (subject: string) => `Ouvrir dans Moodle\u00a0: ${subject}`,
    markRead: "Marquer comme lue",
    markReadLabel: (subject: string) => `Marquer comme lue\u00a0: ${subject}`,
    markAll: "Tout marquer comme lu",
    unreadState: "Non lue",
    messages: (count: number, site: string) =>
      `${formatNumber("fr", count)} ${plural("fr", count) === "one" ? "conversation non lue" : "conversations non lues"} dans ${site}`,
    messagesHint: "Les conversations se lisent dans Moodle.",
    openMessages: "Ouvrir les messages",
    footnote:
      "MoodBuro conserve sur ce Mac les objets des 100 dernières notifications de chaque organisation. En ouvrir une affiche la page des notifications de cette organisation dans Moodle.",
  },
  offline: {
    files: "Fichiers…",
    filesLabel: (name: string) => `Fichiers hors connexion de ${name}`,
    title: "Fichiers du cours",
    back: "Retour aux cours",
    openCourse: "Ouvrir le cours dans Moodle",
    hint: "Enregistrez des fichiers choisis dans les ressources et dossiers Moodle. Les copies s’ouvrent sans connexion. Le chargement et l’actualisation nécessitent une connexion et l’autorisation de télécharger. L’actualisation est manuelle et à sens unique\u00a0: les fichiers modifiés sont enregistrés à côté des copies existantes, les modifications locales sont conservées et les suppressions Moodle ne suppriment aucun fichier local. Retirer un compte conserve les fichiers dans le dossier du cours du Finder mais efface cette liste. Il ne s’agit pas d’un cours hors connexion complet.",
    savedTitle: "Enregistrés sur ce Mac",
    availableTitle: "Disponibles dans Moodle",
    load: "Charger la liste des fichiers",
    emptySaved:
      "Aucune copie enregistrée pour ce cours. Chargez la liste pour choisir des fichiers.",
    noResources:
      "Aucun fichier pris en charge n’a été renvoyé. Ouvrez le cours dans Moodle pour trouver les autres ressources.",
    missing:
      "Cette copie a été déplacée ou est indisponible. Actualisez pour en enregistrer une nouvelle.",
    refresh: "Actualiser la copie",
    refreshLabel: (name: string) =>
      `Actualiser la copie hors connexion de ${name}`,
    unknownSize: "Taille inconnue",
    bytes: (bytes: string) => `${bytes} octets`,
    tooLarge:
      "Dépasse la limite de 128 Mo. Téléchargez ce fichier depuis Moodle.",
    limit:
      "50 fichiers maximum par sélection, 128 Mo chacun. Un transfert à la fois, avec au moins 144 Mo libres. Les 2 000 dernières copies sont affichées. Les transferts s’arrêtent en quittant l’app.",
    saveSelected: (count: number) => `Enregistrer la sélection (${count})`,
    working: "Traitement des fichiers du cours…",
    loading: "Chargement des copies enregistrées…",
    progress: (bytes: string) =>
      `${bytes} octets reçus pour le fichier en cours.`,
    results: "Résultats des transferts",
    file: "Fichier sélectionné",
    unchanged: "Inchangé — copie existante conservée.",
    saved: "Nouvelle copie enregistrée.",
    skipped: (count: number) =>
      `${count} entrées non prises en charge ont été omises. Seuls les fichiers simples des ressources et dossiers Moodle sont pris en charge.`,
  },
  courses: {
    refreshActivities: "Actualiser les activités",
    title: "Cours",
    searchLabel: "Rechercher des cours",
    searchPlaceholder: "Rechercher des cours",
    pinned: "Épinglés",
    pin: "Épingler",
    unpin: "Désépingler",
    pinLabel: (name: string) => `Épingler ${name}`,
    unpinLabel: (name: string) => `Désépingler ${name}`,
    open: "Ouvrir",
    openLabel: (name: string) => `Ouvrir ${name} dans Moodle`,
    count: (count: number) => `${formatNumber("fr", count)} cours`,
    empty: "Aucun cours pour le moment",
    emptyHint:
      "Les cours auxquels vous êtes inscrit apparaissent ici après une actualisation.",
    noMatch: (query: string) =>
      `Aucun cours ne correspond à «\u00a0${query}\u00a0»`,
    noMatchHint:
      "La recherche porte sur les noms de cours et d’activités enregistrés sur ce Mac.",
    footnote:
      "Les épingles sont enregistrées sur ce Mac uniquement et ne modifient pas vos favoris Moodle. Cette liste reste disponible hors ligne\u00a0; ouvrir un cours nécessite une connexion.",
  },
  reminders: {
    title: "Rappels locaux des échéances",
    day: "24 heures avant",
    hour: "1 heure avant",
    label: (site: string, offset: string) => `${site}\u00a0: ${offset}`,
    hint: "Désactivés par défaut. Ces options concernent les échéances à venir de chaque organisation, indépendamment des filtres de l’agenda. MoodBuro actualise périodiquement les calendriers activés et utilise des données de moins de 15 minutes. Les évènements signalés comme non réalisables ou sans élément restant sont ignorés\u00a0; les autres états d’achèvement peuvent être inconnus.",
    limits:
      "Les rappels fonctionnent tant que MoodBuro est en cours d’exécution, même fenêtre fermée. Au démarrage, au réveil ou après un changement d’horloge, les rappels manqués sont ignorés. Aucune garantie lorsque l’app est quittée ou hors connexion. Les autorisations macOS et Concentration peuvent masquer les notifications\u00a0; vérifiez Réglages Système → Notifications. L’app ne peut pas vérifier qu’une notification a été affichée.",
    never:
      "Aucune actualisation de l’agenda réussie. Activez un rappel pour en demander une.",
    updated: (date: string) => `Agenda actualisé le ${date}`,
    stale:
      "Les données de l’agenda sont anciennes. Les rappels attendent une actualisation réussie.",
    deliveryFailed:
      "Une tentative de notification a échoué. Vérifiez les réglages de notifications macOS. Les tentatives échouées ou interrompues ne sont pas rejouées.",
    saving: "Enregistrement des préférences de rappel…",
  },
  agenda: {
    title: "À venir",
    export: "Exporter le calendrier…",
    exporting: "Exportation…",
    from: "Du",
    through: "Au",
    invalidRange:
      "Choisissez des dates valides, avec une fin égale ou postérieure au début.",
    exportHint:
      "L’export enregistre l’agenda filtré en cache dans Téléchargements et affiche le fichier dans le Finder. Importez vous-même le fichier .ics dans Calendrier. Cet instantané ne s’actualise pas automatiquement\u00a0; des imports répétés peuvent créer des doublons.",
    exported: (count: number, filename: string) =>
      `${count} évènements enregistrés dans Téléchargements\u00a0: ${filename}.`,
    revealFailed:
      "Le Finder n’a pas pu afficher le fichier. Retrouvez-le dans votre dossier Téléchargements.",
    count: (count: number) =>
      `${formatNumber("fr", count)} ${plural("fr", count) === "one" ? "évènement" : "évènements"}`,
    empty: "Rien de prévu",
    emptyHint:
      "Les échéances et évènements de vos calendriers Moodle apparaissent ici.",
    kinds: {
      due: "À rendre",
      open: "Ouverture",
      user: "Personnel",
      site: "Site",
      course: "Cours",
      other: "Évènement",
    },
    until: (time: string) => `jusqu’à ${time}`,
    open: "Ouvrir",
    openLabel: (name: string) => `Ouvrir ${name} dans le calendrier Moodle`,
    timeZone: (zone: string) =>
      `Les heures sont affichées dans le fuseau horaire de votre Mac (${zone}).`,
    footnote:
      "Cet agenda est en lecture seule et reflète ce que Moodle a indiqué lors de la dernière actualisation. Moodle reste la référence pour les dépôts et l’achèvement.",
  },
  quickOpen: {
    label: "Ouverture rapide",
    placeholder: "Ouvrir un cours, une activité, une organisation ou une vue",
    cacheDate: (date: string) =>
      `Métadonnées enregistrées · actualisation la plus ancienne\u00a0: ${date}. Les pages Moodle nécessitent une connexion.`,
    cacheEmpty:
      "Aucune métadonnée enregistrée. Actualisez les cours et activités dans Cours.",
    results: "Résultats",
    courses: "Cours",
    organisations: "Organisations",
    views: "Vues",
    pinned: "Épinglé",
    empty: "Aucun résultat",
    emptyHint:
      "La recherche porte sur les noms de cours et d’activités enregistrés sur ce Mac.",
    hint: "↑ ↓ pour choisir · Retour pour ouvrir · Échap pour fermer",
  },
  toolbar: {
    inbox: "Boîte de réception",
    courses: "Cours",
    agenda: "Agenda",
    quickOpen: "Ouverture rapide (⌘L)",
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
    preview: "Aperçu",
    previewLabel: (name: string) => `Afficher un aperçu de ${name}`,
    previewHint:
      "Les fichiers PDF, Word, Excel et PowerPoint utilisent le même lecteur natif macOS. Utilisez Ouvrir pour les modifier dans l’application par défaut. Les aperçus nécessitent l’app de bureau.",
    courseFilter: "Cours",
    allCourses: "Tous les cours",
    unknownCourse: "Cours inconnu",
    refreshFolders: "Actualiser les dossiers de cours",
    returnToMoodle: "Retour à Moodle",
    returnHint:
      "Après un échec, retournez dans Moodle pour relancer le téléchargement. Cette action ne relance pas le transfert.",
    foldersRefreshed:
      "Dossiers de cours actualisés. Les nouveaux téléchargements seront classés par cours identifié\u00a0; les fichiers existants restent en place.",
    openLabel: (name: string) => `Ouvrir ${name}`,
    revealLabel: (name: string) => `Afficher ${name} dans le Finder`,
    title: "Téléchargements",
    searchLabel: "Rechercher dans les téléchargements",
    searchPlaceholder: "Rechercher par nom de fichier",
    stateFilter: "État",
    allStates: "Tous",
    filters: {
      downloading: "En cours",
      complete: "Terminés",
      failed: "Échecs",
    },
    shown: (shown: number, total: number) =>
      `${formatNumber("fr", shown)} sur ${formatNumber("fr", total)}`,
    showMore: "Afficher plus",
    noMatch: "Aucun téléchargement ne correspond",
    noMatchHint:
      "Essayez un autre nom, une autre organisation ou un autre état.",
    emptyHint: "Les fichiers téléchargés depuis Moodle sont listés ici.",
    copyPath: "Copier le chemin",
    copyPathLabel: (name: string) => `Copier le chemin de ${name}`,
    copied: "Chemin copié dans le presse-papiers.",
    more: (name: string) => `Autres actions pour ${name}`,
    clear: "Effacer l’historique…",
    clearTitle: "Effacer l’historique des téléchargements\u00a0?",
    clearTitleFor: (site: string) =>
      `Effacer l’historique des téléchargements de ${site}\u00a0?`,
    clearBody:
      "Seule cette liste est effacée. Les fichiers téléchargés restent dans ~/Moodle, et les transferts en cours restent listés.",
    confirmClear: "Effacer l’historique",
    cleared: "Historique effacé. Vos fichiers ont été conservés.",
    empty: "Aucun téléchargement pour le moment",
    loading: "Chargement des téléchargements…",
    location:
      "Les fichiers sont enregistrés dans ~/Moodle, par organisation et cours identifié. Actualisez les dossiers pour indexer les ressources Moodle. Choisissez une organisation pour filtrer par cours. Les fichiers non identifiés restent dans Downloads. Retirer une organisation conserve ses fichiers.",
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
      "Aperçu dans le navigateur\u00a0: les changements de réglages durent jusqu’au rechargement, et les comptes ne peuvent être gérés que dans l’app de bureau.",
    general: "Général",
    shortLabel: "Abréviation",
    shortLabelHint:
      "4 lettres ou chiffres maximum\u00a0; laisser vide pour les initiales automatiques.",
    saveLabel: "Enregistrer",
    labelSaved: "Enregistré",
    theme: "Thème",
    themeHint: "Automatique suit le réglage d’apparence de votre Mac.",
    appearance: { system: "Automatique", light: "Clair", dark: "Sombre" },
    language: "Langue",
    languageHint: "Automatique suit la langue préférée de votre Mac.",
    languages: { system: "Automatique", fr: "Français", en: "English" },
    globalShortcut: "Ouverture rapide depuis n’importe quelle app",
    globalShortcutHint:
      "⌘⇧M affiche MoodBuro et l’ouverture rapide tant que l’app est lancée.",
    shortcutFootnote:
      "macOS ne signale pas toujours un conflit de raccourci\u00a0: si une autre app utilise déjà ⌘⇧M, elle peut le conserver.",
    shortcutOptions: { off: "Désactivé", on: "Activé" },
    shortcutUnavailable:
      "⌘⇧M n’a pas pu être enregistré au lancement de MoodBuro. Désactivez-le puis réactivez-le pour réessayer.",
    files: "Fichiers",
    pdf: "Documents PDF",
    pdfHint: "Ouverts directement par une page Moodle.",
    images: "Images",
    imagesHint: "Ouvertes directement par une page Moodle.",
    handling: {
      inline: "Afficher dans Moodle",
      save: "Enregistrer",
      open: "Enregistrer et ouvrir",
    },
    filesFootnote:
      "Les fichiers enregistrés vont dans le dossier du cours, ou dans Téléchargements si le cours est inconnu. «\u00a0Enregistrer et ouvrir\u00a0» ouvre ensuite le fichier dans son application par défaut. Les fichiers intégrés à une page Moodle restent dans la page. Pour enregistrer le fichier affiché, choisissez Fichier › Enregistrer dans Téléchargements (⌘S).",
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
    moveUp: (name: string) => `Monter ${name}`,
    moveDown: (name: string) => `Descendre ${name}`,
    orderHint:
      "Les flèches changent l’ordre des organisations dans la barre latérale et la barre des menus. Cet ordre est enregistré sur ce Mac.",
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
