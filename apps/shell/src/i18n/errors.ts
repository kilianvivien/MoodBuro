import type { Locale } from "./locale";
import { catalogues, type Messages } from "./messages";

/**
 * French for every error string Rust can return today (src-tauri/src/commands.rs,
 * windows.rs, lib.rs, md-moodle::Error) plus the browser-preview message from
 * ipc.ts. Rust errors are English strings for now, so they are translated here by
 * exact text. Keep this list in step with the Rust messages.
 */
const french: Record<string, string> = {
  // AI assistant (src-tauri/src/assistant.rs, md-assistant::Error)
  "The AI provider rejected this API key. Check the key in Settings.":
    "Le fournisseur d’IA a refusé cette clé d’API. Vérifiez la clé dans les Réglages.",
  "This API key is not allowed to use the selected model or service.":
    "Cette clé d’API n’est pas autorisée à utiliser le modèle ou le service sélectionné.",
  "The AI provider is limiting requests or your quota is used up. Try again later.":
    "Le fournisseur d’IA limite les requêtes ou votre quota est épuisé. Réessayez plus tard.",
  "The selected AI model is not available. Choose another model in Settings.":
    "Le modèle d’IA sélectionné n’est pas disponible. Choisissez-en un autre dans les Réglages.",
  "The AI provider refused this request. Try another model in Settings.":
    "Le fournisseur d’IA a refusé cette requête. Essayez un autre modèle dans les Réglages.",
  "The AI provider could not be reached. Check your connection and try again.":
    "Le fournisseur d’IA est injoignable. Vérifiez votre connexion puis réessayez.",
  "The AI provider returned an unexpected response.":
    "Le fournisseur d’IA a renvoyé une réponse inattendue.",
  "The assistant is still answering. Wait for it to finish or stop it.":
    "L’assistant est encore en train de répondre. Attendez la fin ou arrêtez-le.",
  "This conversation belongs to another account. Reopen the assistant for the current account.":
    "Cette conversation appartient à un autre compte. Rouvrez l’assistant pour le compte actuel.",
  "Add an API key for the AI provider in Settings.":
    "Ajoutez une clé d’API pour le fournisseur d’IA dans les Réglages.",
  "Allow sharing Moodle context with the AI provider in Settings first.":
    "Autorisez d’abord le partage du contexte Moodle avec le fournisseur d’IA dans les Réglages.",
  "Could not access the AI provider key. Unlock the OS credential store and try again.":
    "Impossible d’accéder à la clé du fournisseur d’IA. Déverrouillez le trousseau puis réessayez.",
  "The assistant is unavailable. Restart MoodBuro.":
    "L’assistant est indisponible. Relancez MoodBuro.",
  "Unknown AI provider.": "Fournisseur d’IA inconnu.",
  "Enter a model name using letters, numbers and - _ . / : only.":
    "Saisissez un nom de modèle composé uniquement de lettres, de chiffres et de - _ . / :",
  "This does not look like an API key. Paste the key without spaces.":
    "Ceci ne ressemble pas à une clé d’API. Collez la clé sans espaces.",
  "Could not save the API key in the OS credential store.":
    "Impossible d’enregistrer la clé d’API dans le trousseau.",
  "Could not remove the API key from the OS credential store.":
    "Impossible de supprimer la clé d’API du trousseau.",
  "This confirmation is no longer valid.":
    "Cette confirmation n’est plus valide.",
  "Invalid assistant request.": "Requête de l’assistant invalide.",
  "Open an organisation before attaching its context.":
    "Ouvrez une organisation avant de joindre son contexte.",
  "There is no unanswered message to retry.":
    "Aucun message sans réponse à relancer.",
  "Enter a message of up to 8,000 characters.":
    "Saisissez un message de 8 000 caractères au maximum.",
  "Unknown assistant context.": "Contexte de l’assistant inconnu.",
  "The assistant needed too many steps. Ask a narrower question.":
    "L’assistant a eu besoin de trop d’étapes. Posez une question plus précise.",
  "Could not read reminder settings.":
    "Impossible de lire les préférences de rappel.",
  "Could not save reminder settings. Reopen the agenda for the current account.":
    "Impossible d’enregistrer les préférences de rappel. Rouvrez l’agenda du compte actuel.",
  "This agenda belongs to another account. Reopen the agenda.":
    "Cet agenda appartient à un autre compte. Rouvrez l’agenda.",
  "This resource cannot be saved offline.":
    "Cette ressource ne peut pas être enregistrée hors connexion.",
  "Could not download this resource.":
    "Impossible de télécharger cette ressource. Réessayez avec une connexion active.",
  "This resource exceeds the 128 MB offline limit.":
    "Cette ressource dépasse la limite de 128 Mo hors connexion.",
  "Could not save this resource.":
    "Impossible d’enregistrer cette ressource. Vérifiez l’espace disque et les autorisations du dossier.",
  "This offline selection belongs to another account. Reopen the course files.":
    "Cette sélection appartient à un autre compte. Rouvrez les fichiers du cours.",
  "The course returned ambiguous file metadata.":
    "Le cours a renvoyé des métadonnées de fichiers ambiguës.",
  "This course has too many files. Save files from Moodle instead.":
    "Ce cours contient trop de fichiers. Enregistrez-les depuis Moodle.",
  "Could not read offline files.":
    "Impossible de lire les fichiers hors connexion.",
  "Could not safely publish this offline file.":
    "Impossible de finaliser ce fichier hors connexion en toute sécurité.",
  "Select between 1 and 50 files to save offline.":
    "Sélectionnez entre 1 et 50 fichiers à enregistrer hors connexion.",
  "Another offline transfer is running. Wait for it to finish.":
    "Un autre transfert hors connexion est en cours. Attendez sa fin.",
  "This resource is no longer available. Reload the course file list.":
    "Cette ressource n’est plus disponible. Rechargez la liste des fichiers du cours.",
  "Could not check available disk space.":
    "Impossible de vérifier l’espace disque disponible.",
  "Not enough disk space. Free at least 144 MB and try again.":
    "Espace disque insuffisant. Libérez au moins 144 Mo puis réessayez.",
  "The file was saved, but its offline index could not be updated. Find it in the course folder.":
    "Le fichier a été enregistré, mais son index hors connexion n’a pas pu être actualisé. Retrouvez-le dans le dossier du cours.",
  "This offline file is unavailable. Save a fresh copy from the course.":
    "Ce fichier hors connexion est indisponible. Enregistrez une nouvelle copie depuis le cours.",
  "Could not open this offline file.":
    "Impossible d’ouvrir ce fichier hors connexion.",

  "Could not reorder organisations.":
    "Impossible de réordonner les organisations.",
  "Invalid calendar date range.":
    "La plage de dates du calendrier est invalide.",
  "Invalid calendar event.": "Un évènement du calendrier est invalide.",
  "No calendar events match these filters.":
    "Aucun évènement ne correspond à ces filtres.",
  "Could not export the calendar.": "Impossible d’exporter le calendrier.",
  "The ⌘⇧M shortcut could not be registered. Another app or a system shortcut may already use it.":
    "Le raccourci ⌘⇧M n’a pas pu être enregistré. Une autre app ou un raccourci système l’utilise peut-être déjà.",
  "The ⌘⇧M shortcut could not be turned off.":
    "Le raccourci ⌘⇧M n’a pas pu être désactivé.",
  "This page is not a Moodle file. Use the file’s own download link.":
    "Cette page n’est pas un fichier Moodle. Utilisez le lien de téléchargement du fichier.",
  "Could not save this file.": "Impossible d’enregistrer ce fichier.",
  "Could not preview this document.":
    "Impossible d’afficher l’aperçu de ce document.",
  "This document type cannot be previewed.":
    "Ce type de document ne peut pas être affiché en aperçu.",
  "Close a document preview before opening another.":
    "Fermez un aperçu de document avant d’en ouvrir un autre.",
  "Document preview is currently available on macOS. Use Open in Default App.":
    "L’aperçu est disponible sur macOS. Ouvrez le fichier dans l’application par défaut.",
  "Could not save course folders.":
    "Impossible d’enregistrer les dossiers de cours.",

  "This Moodle link is invalid or does not match a saved organisation.":
    "Ce lien Moodle est invalide ou ne correspond pas à une organisation enregistrée.",
  "This file was requested twice. Restart MoodBuro before downloading it again.":
    "Ce fichier a été demandé deux fois. Relancez MoodBuro avant de le télécharger à nouveau.",
  "Download folder unavailable.": "Dossier de téléchargement indisponible.",
  "Download unavailable.": "Téléchargement indisponible.",
  "This file is already downloading or the download queue is full.":
    "Ce fichier est déjà en cours de téléchargement ou la file est pleine.",
  "This download is outside the organisation’s secure site.":
    "Ce téléchargement se trouve en dehors du site sécurisé de l’organisation.",
  "Could not save download history.":
    "Impossible d’enregistrer l’historique des téléchargements.",
  "Could not read download history.":
    "Impossible de lire l’historique des téléchargements.",
  "This download is unavailable.": "Ce téléchargement est indisponible.",
  "Confirm opening this file first.":
    "Confirmez d’abord l’ouverture de ce fichier.",
  "Could not open the downloaded file.":
    "Impossible d’ouvrir le fichier téléchargé.",
  "Could not reveal the downloaded file.":
    "Impossible d’afficher le fichier téléchargé dans le Finder.",
  "Could not prepare PDF saving.":
    "Impossible de préparer l’enregistrement des PDF.",
  "This PDF could not be saved. Use Moodle’s file download link.":
    "Impossible d’enregistrer ce PDF. Utilisez le lien de téléchargement du fichier dans Moodle.",
  "This command is restricted to the popup toolbar.":
    "Cette commande est réservée à la barre d’outils de la fenêtre Moodle.",
  "Close a Moodle popup window before opening another.":
    "Fermez une fenêtre Moodle avant d’en ouvrir une autre.",
  // md-moodle::Error
  "Enter an HTTPS Moodle base URL, without a query or fragment.":
    "Saisissez l’adresse HTTPS de base du Moodle, sans paramètres ni fragment.",
  "The sign-in response is invalid or belongs to another sign-in attempt.":
    "La réponse de connexion est invalide ou appartient à une autre tentative de connexion.",
  "The Moodle site could not be reached or returned an unexpected response.":
    "Le site Moodle est injoignable ou a renvoyé une réponse inattendue.",
  "This site could not authorise mobile web services. Ask its administrator to enable them.":
    "Ce site n’a pas pu autoriser les services web mobiles. Demandez à son administrateur de les activer.",
  "Your Moodle session has expired. Reconnect this organisation in Settings.":
    "Votre session Moodle a expiré. Reconnectez cette organisation dans les Réglages.",
  "Moodle is limiting requests. Please try again shortly.":
    "Moodle limite les requêtes. Réessayez dans quelques instants.",
  "This account does not have permission for that Moodle service.":
    "Ce compte n’a pas l’autorisation d’utiliser ce service Moodle.",

  // Sign-in and organisations
  "Sign-in cancelled.": "Connexion annulée.",
  "The site reports a different base origin. Enter its canonical Moodle URL.":
    "Le site indique une autre origine. Saisissez son adresse Moodle officielle.",
  "Invalid sign-in URL.": "Adresse de connexion invalide.",
  "The site supplied an unsafe sign-in URL.":
    "Le site a fourni une adresse de connexion non sécurisée.",
  "The alpha supports up to 10 sites.":
    "La version alpha prend en charge 10 sites au maximum.",
  "This Moodle site has already been added.":
    "Ce site Moodle a déjà été ajouté.",
  "Finish or close the current sign-in window first.":
    "Terminez ou fermez d’abord la fenêtre de connexion en cours.",
  "Finish or close the current sign-in window before removing an organisation.":
    "Terminez ou fermez la fenêtre de connexion en cours avant de retirer une organisation.",
  "Could not open the organisation sign-in window.":
    "Impossible d’ouvrir la fenêtre de connexion de l’organisation.",
  "Sign-in state unavailable.": "État de connexion indisponible.",
  "Could not access the existing credential. Unlock the OS credential store and try again.":
    "Impossible d’accéder à l’identifiant existant. Déverrouillez le trousseau et réessayez.",
  "Could not access the existing private credential.":
    "Impossible d’accéder à l’identifiant privé existant.",
  "Could not save the token in the OS credential store.":
    "Impossible d’enregistrer le jeton dans le trousseau.",
  "Could not save the private token in the OS credential store.":
    "Impossible d’enregistrer le jeton privé dans le trousseau.",
  "Could not update the private credential.":
    "Impossible de mettre à jour l’identifiant privé.",
  "Could not save the site metadata.":
    "Impossible d’enregistrer les informations du site.",
  "Reconnect this organisation in Settings to restore its credentials.":
    "Reconnectez cette organisation dans les Réglages pour rétablir ses identifiants.",
  "Could not remove this organisation’s credentials. Please try again.":
    "Impossible de supprimer les identifiants de cette organisation. Réessayez.",
  "Could not remove the saved organisation.":
    "Impossible de retirer l’organisation enregistrée.",

  // Storage and state
  "This command is restricted to the local shell.":
    "Cette commande est réservée à l’interface locale.",
  "Site storage unavailable.": "Stockage des sites indisponible.",
  "Could not read sites.": "Impossible de lire les sites.",
  "Could not read saved sites.": "Impossible de lire les sites enregistrés.",
  "Unknown site.": "Site inconnu.",
  "Invalid site identifier.": "Identifiant de site invalide.",
  "Invalid site URL.": "Adresse de site invalide.",
  "Invalid site host.": "Hôte du site invalide.",
  "Site state unavailable.": "État du site indisponible.",
  "Settings unavailable.": "Réglages indisponibles.",
  "Could not read settings.": "Impossible de lire les réglages.",
  "Could not save settings.": "Impossible d’enregistrer les réglages.",
  "Could not change the window appearance.":
    "Impossible de modifier l’apparence de la fenêtre.",

  // Views and navigation
  "Unknown shell view.": "Vue inconnue.",
  "Could not show the local shell.":
    "Impossible d’afficher l’interface locale.",
  "Local shell missing.": "Interface locale introuvable.",
  "Main window missing.": "Fenêtre principale introuvable.",
  "Moodle view missing.": "Vue Moodle introuvable.",
  "Could not create the Moodle view.": "Impossible de créer la vue Moodle.",
  "Could not resize the Moodle view.":
    "Impossible de redimensionner la vue Moodle.",
  "Could not show the Moodle view.": "Impossible d’afficher la vue Moodle.",
  "Could not renew the Moodle view.": "Impossible de renouveler la vue Moodle.",
  "Could not close the Moodle view.": "Impossible de fermer la vue Moodle.",
  "Could not switch sites.": "Impossible de changer de site.",
  "Could not access the site’s browser data.":
    "Impossible d’accéder aux données de navigation du site.",
  "Could not hide the browser data view.":
    "Impossible de masquer la vue des données de navigation.",
  "Could not clear the site’s browser data.":
    "Impossible d’effacer les données de navigation du site.",
  "Could not read browser cookies.":
    "Impossible de lire les cookies du navigateur.",
  "Could not remove a site cookie.":
    "Impossible de supprimer un cookie du site.",
  "Choose a site first.": "Choisissez d’abord un site.",
  "Could not read the current page.": "Impossible de lire la page actuelle.",
  "This sign-in page cannot be shared with the browser. Open the organisation’s base URL instead.":
    "Cette page de connexion ne peut pas être ouverte dans le navigateur. Ouvrez plutôt l’adresse de base de l’organisation.",
  "Could not open the browser.": "Impossible d’ouvrir le navigateur.",
  "Unknown navigation action.": "Action de navigation inconnue.",
  "Navigation failed.": "La navigation a échoué.",

  // Inbox, courses and agenda
  "Could not read course activities.":
    "Impossible de lire les activités des cours.",
  "Could not save course activities.":
    "Impossible d’enregistrer les activités des cours.",
  "This activity is no longer available. Refresh your course activities.":
    "Cette activité n’est plus disponible. Actualisez les activités de vos cours.",
  "Invalid activity metadata.":
    "Les métadonnées de cette activité sont invalides.",
  "Unknown Moodle page.": "Page Moodle inconnue.",
  "This course is no longer in your list. Refresh your courses.":
    "Ce cours ne figure plus dans votre liste. Actualisez vos cours.",
  "Could not read the course list.": "Impossible de lire la liste des cours.",
  "Could not save the course list.":
    "Impossible d’enregistrer la liste des cours.",
  "Could not update this pin.": "Impossible de modifier cette épingle.",
  "Could not read notifications.": "Impossible de lire les notifications.",
  "Could not update notifications.":
    "Impossible de mettre à jour les notifications.",
  "This notification is no longer available.":
    "Cette notification n’est plus disponible.",
  "Could not read the agenda.": "Impossible de lire l’agenda.",
  "Could not save the agenda.": "Impossible d’enregistrer l’agenda.",
  "Unknown download filter.": "Filtre de téléchargement inconnu.",
  "Could not clear download history.":
    "Impossible d’effacer l’historique des téléchargements.",
  "Could not copy the file path.": "Impossible de copier le chemin du fichier.",

  "Use up to 4 letters or numbers for the short label.":
    "Utilisez au maximum 4 lettres ou chiffres pour l’abréviation.",

  // Browser preview (ipc.ts)
  "Open the desktop app with pnpm tauri dev to manage sign-in.":
    "Ouvrez l’app de bureau avec pnpm tauri dev pour gérer les connexions.",
  "Open the desktop app with pnpm tauri dev to use this feature.":
    "Ouvrez l’app de bureau avec pnpm tauri dev pour utiliser cette fonction.",
};

/** What was being attempted, so an untranslated error still says what failed. */
export type ErrorContext = keyof Omit<Messages["errors"], "detail">;

export interface DescribedError {
  /** Text in the UI language. */
  text: string;
  /** The original backend message, kept when it could not be translated. */
  detail: string | null;
}

/** Straight vs typographic apostrophes and stray whitespace must not defeat a match. */
const normalize = (raw: string) => raw.trim().replace(/'/gu, "’");
const lookup = new Map(
  Object.entries(french).map(([english, text]) => [normalize(english), text]),
);

export function knownErrors() {
  return Object.keys(french);
}

export function describeError(
  locale: Locale,
  raw: string,
  context: ErrorContext = "general",
): DescribedError {
  const original = raw.trim();
  if (locale === "en")
    return { text: original || catalogues.en.errors[context], detail: null };
  const translated = lookup.get(normalize(original));
  if (translated) return { text: translated, detail: null };
  // Never show unexplained English in a French UI, but keep the backend's
  // wording as a labelled detail so the meaning is not lost.
  return {
    text: catalogues.fr.errors[context],
    detail: original || null,
  };
}
