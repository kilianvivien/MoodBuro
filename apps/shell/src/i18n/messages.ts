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
    inbox: "Notifications",
    courses: "Courses",
    agenda: "Agenda",
    views: "Views",
    organisations: "Organisations",
  },
  common: {
    details: "Details",
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
  sync: {
    title: "Automatic Offline Refresh",
    selected: "Selected for automatic refresh",
    hint: "Opt in for selected files in this course. Refresh runs only while MoodBuro is open, one file at a time, with retry backoff. Saved copies remain usable after failures. Unfinished files restart after cancellation or quit; completed files are kept. Remote deletions and renames never remove local files.",
    schedule: "Refresh",
    paused: "Paused",
    hourly: "Every Hour",
    daily: "Every Day",
    cancel: "Cancel Current Refresh",
    now: "Refresh Now",
    enable: "Refresh Selected Files Daily",
    replaceSelection: "Use Selected Files for Automatic Refresh",
    selectionHint:
      "Select 1–50 files from the Moodle list below. Applying a new selection replaces this course’s schedule selection and stops any current attempt. Files no longer selected stay on the Mac. Browser demo records preferences but does not run background transfers.",
    count: (total: number, pending: number) =>
      `${total} selected · ${pending} remaining`,
    next: (date: string) => `Next attempt while the app is open: ${date}`,
    missing: (count: number) =>
      `${count} selected files are no longer returned by Moodle. They may have been renamed, moved or removed, or access may have changed. Saved copies are kept. Load the file list to review the selection.`,
    states: {
      paused: "Paused",
      waiting: "Waiting to refresh",
      running: "Refreshing selected files…",
      idle: "Refresh complete",
      error: "Refresh failed; unfinished files will retry",
      cancelled:
        "Current refresh cancelled; the next scheduled refresh remains enabled",
      interrupted: "Recovering unfinished files after restart",
    },
  },
  offline: {
    files: "Files…",
    filesLabel: (name: string) => `Offline files for ${name}`,
    title: "Course files",
    back: "Back to Courses",
    openCourse: "Open Course in Moodle",
    hint: "Save selected plain files from Moodle resources and folders. Saved copies open without a connection. Loading and refreshing need a connection and permission to download files. Refresh is one-way: changed files are saved alongside existing copies, local edits are kept, and Moodle deletions never delete local files. Account removal keeps files in the course’s Finder folder but clears this list. This is not a complete offline course.",
    remove: "Remove Copy…",
    confirmRemove: "Remove Copy",
    removeLabel: (name: string) => `Remove saved copy of ${name}`,
    removeTitle: (name: string, site: string) =>
      `Remove “${name}” from ${site}?`,
    removeBody:
      "Only this saved version will leave the offline list. An unchanged file moves to ~/Moodle/.trash, where it stays until you remove it in Finder. A locally edited file stays in its current folder. A file shared with another saved entry or download also stays in place. Other versions and Moodle files are kept. This does not free disk space immediately. An automatic refresh selection may save this file again; pause or change the selection to prevent that.",
    removed: {
      archived:
        "Copy removed. The unchanged file is recoverable in ~/Moodle/.trash (Finder → Go → Go to Folder). No automatic cleanup is applied.",
      kept: "Entry removed. The file was edited, shared with another saved entry or download, or could not be safely moved, so it was kept in its current folder.",
      missing: "Unavailable copy removed from the list. No file was moved.",
    },
    available: "Available offline",
    notSaved: "Not saved on this Mac",
    copyMissing: "Saved copy unavailable",
    lastRefreshed: (date: string) => `Last successful refresh: ${date}`,
    freshnessHint:
      "Refresh manually or enable a schedule for selected files. Moodle files may have changed since the last successful refresh. Local edits and previous versions are kept.",
    refreshFailed:
      "Refresh failed. You can still open this saved copy; it may differ from Moodle.",
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
    summary:
      "Choose when to be notified before deadlines for each organisation.",
    help: "How reminders work",
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
  calendar: {
    title: "Calendar Subscription",
    scope: "Read-only · past 5 days and next 60 days",
    setup: "Set Up…",
    again: "Set Up Again…",
    confirm: "Open Calendar",
    connecting: "Checking…",
    moodle: "Moodle Export…",
    consentTitle: (name: string) => `Subscribe to ${name}?`,
    consent:
      "Calendar will store a private feed link. Removing this account from MoodBuro will not revoke it.",
    duplicate: "Check Calendar first to avoid a duplicate subscription.",
    requested: (date: string) => `Setup requested ${date}`,
    sent: "Finish setup and choose a refresh interval in Calendar.",
    details:
      "Remove subscriptions in Calendar; ask your Moodle administrator to revoke access. MoodBuro reminders require the app to stay open.",
    demo: "Preview only; no calendar will be added.",
    demoSent: "Setup preview recorded.",
  },
  agenda: {
    help: "Calendar export and details",
    reminders: "Reminder Settings…",
    remind: {
      day: (name: string) => `Remind me 24 hours before: ${name}`,
      hour: (name: string) => `Remind me 1 hour before: ${name}`,
      onDay: (name: string) => `Reminder set 24 hours before: ${name}. Remove`,
      onHour: (name: string) => `Reminder set 1 hour before: ${name}. Remove`,
    },
    dates: "Filter Dates…",
    clearDates: "All Dates",
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
    timeZone: (zone: string) => `Time zone: ${zone}`,
    footnote:
      "Moodle controls which upcoming events appear here. Completion information and reminder eligibility may be incomplete.",
  },
  bookmarks: {
    title: "Bookmarks",
    newTitle: "New bookmark",
    editTitle: "Edit bookmark",
    saved: "Bookmark saved.",
    saving: "Saving…",
    removeQuestion: "Remove this bookmark?",
    add: "Bookmark This Page… (⌘D)",
    more: "More bookmarks",
    edit: "Edit bookmark",
    remove: "Remove bookmark",
    name: "Name",
    icon: "Icon",
    save: "Save",
    cancel: "Cancel",
    close: "Close bookmarks",
    empty: "Bookmark a Moodle page using the bookmark button in the toolbar.",
    local: "Saved on this Mac for this organisation and account.",
    icons: {
      bookmark: "Bookmark",
      star: "Star",
      book: "Book",
      calendar: "Calendar",
      file: "File",
      folder: "Folder",
      message: "Message",
      graduation: "Graduation cap",
    },
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
    inbox: "Notifications",
    courses: "Courses",
    agenda: "Agenda",
    bookmarks: "Bookmarks",
    quickOpen: "Quick Open (⌘K)",
    sessionWaiting: (time: string) =>
      `Moodle limits automatic sign-in. Retrying at ${time}.`,
    sessionRetrying: "Restoring your Moodle session…",
    sessionFailed:
      "Automatic sign-in failed. Sign in to Moodle or reconnect in Settings.",
    sessionHelp:
      "Moodle temporarily limits automatic sign-in after a recent login (normally about six minutes; your site may use a longer delay). Your credentials are still saved. MoodBuro retries once at the displayed time while running; you can also sign in manually.",
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
    destination:
      "Saved in ~/Moodle, in an organisation folder, then a course folder. Use Show in Finder to see the exact location.",
    folders:
      "Folder names include internal IDs to keep organisations and courses separate. Files without an identified course go into the organisation’s Downloads folder.",
    feedback: {
      downloading: (name: string) => `Downloading ${name}…`,
      complete: (name: string) => `Downloaded: ${name}`,
      failed: (name: string) => `Download failed: ${name}`,
    },
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
      "This organisation’s saved account and Keychain credentials will be removed from this Mac. Its private browser profile will be cleared where supported. On older macOS versions, shared sign-in sessions may remain. Your Moodle account and courses are not affected. Remove calendar subscriptions separately in Calendar.",
    cancel: "Cancel",
    confirmRemove: "Remove",
    removed: (name: string) => `${name} was removed from this Mac.`,
    removedUnnamed: "The site was removed from this Mac.",
    reauthenticated: (name: string) => `Signed in to ${name} again.`,
    reauthenticatedUnnamed: "Signed in to the site again.",
  },
  assistant: {
    title: "AI Assistant",
    toggle: "AI Assistant (⇧⌘A)",
    general: "General chat",
    scope: (name: string) => `Conversation for ${name}`,
    close: "Close Assistant",
    clear: "Clear Conversation…",
    clearTitle: "Clear this conversation?",
    clearBody:
      "The messages shown here will be forgotten. Conversations are kept in memory only and are never saved to disk.",
    clearConfirm: "Clear",
    cancel: "Cancel",
    resize: "Assistant panel width",
    log: "Conversation",
    you: "You",
    answer: "Answer",
    placeholder: "Ask a question",
    inputLabel: "Message to the assistant",
    inputHint: "Return to send · Shift-Return for a new line",
    send: "Send",
    stop: "Stop",
    retry: "Try Again",
    thinking: "Waiting for the answer…",
    steps: {
      courses: "Checking your courses…",
      activities: "Searching saved activities…",
      agenda: "Checking your agenda…",
      notifications: "Checking your notifications…",
      files: "Listing the course files…",
      open: "Opening in Moodle…",
      save: "Saving the file…",
      confirm: "Waiting for your confirmation…",
    },
    contextLabel: "Moodle context",
    contextNone: "No Moodle context",
    contextSite: (name: string) => `Organisation: ${name}`,
    contextCourses: "Courses",
    contextNeedsConsent:
      "Sharing Moodle context is off. Turn it on in Settings to attach an organisation or a course.",
    contextNeedsSite: "Open an organisation to attach its Moodle context.",
    attached: (name: string) => `Context: ${name}`,
    sources: "Sources",
    moreSources: (count: number) => `${formatNumber("en", count)} more`,
    fewerSources: "Show fewer",
    openSource: (label: string) => `Open in Moodle: ${label}`,
    openFile: (label: string) =>
      `Show the copy saved on this Mac, or open in Moodle: ${label}`,
    suggestions: "Suggested questions",
    suggest: {
      due: "What is due this week?",
      news: "What is new in my notifications?",
      files: "Which files does this course have?",
      courses: "What are my courses?",
    },
    setupTitle: "Set up the assistant",
    setupHint:
      "Add your own Albert API or Mistral AI key in Settings to start a conversation.",
    openSettings: "Open Settings…",
    emptyTitle: "Ask a question",
    emptyHint: (provider: string) =>
      `Your messages are sent to ${provider}. The conversation stays in memory until you quit MoodBuro and is never saved to disk.`,
    confirm: {
      "mark-read": {
        title: (): string => "Mark this notification as read?",
        body: (site: string) =>
          `In ${site}. Nothing changes in Moodle unless you confirm.`,
        approve: "Mark as Read",
        decline: "Don’t Mark",
      },
      "save-file": {
        title: (): string => "Save this file to your Mac?",
        body: (site: string, size: string | null) =>
          `From ${site}, into the course folder in your Moodle folder${size ? ` (${size})` : ""}. Nothing changes in Moodle.`,
        approve: "Save",
        decline: "Don’t Save",
      },
      reminder: {
        title: (when: string | null): string =>
          when === "hour"
            ? "Remind you 1 hour before this deadline?"
            : "Remind you 24 hours before this deadline?",
        body: (site: string) =>
          `In ${site}. A notification on this Mac, delivered only while MoodBuro is running. You can remove it in the Agenda.`,
        approve: "Add Reminder",
        decline: "Don’t Add",
      },
    },
    notes: {
      courses: "Checked your course list",
      activities: "Searched activities saved on this Mac",
      agenda: "Checked your agenda",
      notifications: "Checked notification subjects saved on this Mac",
      files: "Listed the course’s file names",
      saved: (label: string) => `Saved to your Moodle folder: ${label}`,
      saveFailed: (label: string) => `Could not save “${label}”.`,
      saveDeclined: (label: string) => `Not saved: ${label}`,
      reminderDay: (label: string) =>
        `Reminder added 24 hours before: ${label}`,
      reminderHour: (label: string) => `Reminder added 1 hour before: ${label}`,
      reminderFailed: (label: string) =>
        `Could not add a reminder for “${label}”.`,
      reminderDeclined: (label: string) => `No reminder added: ${label}`,
      updated: (when: string) => `updated ${when}`,
      updatedNow: "updated just now",
      stale: "could not be refreshed, so saved data was used",
      opened: (label: string) => `Opened in Moodle: ${label}`,
      openedNotifications: "Opened the notifications page in Moodle",
      openedCalendar: "Opened the calendar in Moodle",
      marked: (label: string) => `Marked as read: ${label}`,
      declined: (label: string) => `Left unread: ${label}`,
      writeFailed: (label: string) =>
        `Moodle did not accept marking “${label}” as read.`,
      refused: "A requested action is not available and was skipped.",
      stopped: "Stopped before the answer was complete.",
    },
    settings: {
      title: "AI assistant",
      intro:
        "Optional. Chat in a side panel using your own Albert API or Mistral AI key.",
      provider: "Provider",
      providerHint:
        "Each provider keeps its own key and model. Switching provider clears conversations.",
      key: "API key",
      keySaved: "A key is saved in your Mac’s Keychain.",
      keyMissing: "No key saved yet.",
      keyPlaceholder: "Paste your API key",
      saveKey: "Save Key",
      removeKey: "Remove Key…",
      removeKeyTitle: (provider: string) => `Remove the ${provider} key?`,
      removeKeyBody:
        "The key is deleted from your Mac’s Keychain and any answer in progress stops. The key itself stays valid with the provider.",
      removeKeyConfirm: "Remove",
      model: "Model",
      modelHint:
        "Check the connection to choose from the models your key can use.",
      modelLabel: "Model name",
      saveModel: "Save",
      check: "Check Connection",
      checking: "Checking…",
      connected: (count: number) =>
        `Connected. ${formatNumber("en", count)} chat ${plural("en", count) === "one" ? "model is" : "models are"} available.`,
      consent: "Share Moodle context",
      consentHint: (provider: string) =>
        `Turning this off stops the current answer and clears conversations. Off: the assistant only sees what you type. On: when you attach an organisation or a course to a message, MoodBuro also sends ${provider} the organisation name and the course names, saved activity names, course file names with their section and size, upcoming deadlines and notification subjects the assistant looks up. Sign-in tokens, cookies, API keys, your name, site addresses and file contents are never sent.`,
      history: "Conversations",
      historyHint:
        "Kept in memory only, separately for each organisation account, until you quit MoodBuro. Nothing is written to disk.",
      clearAll: "Delete All Conversations…",
      clearAllTitle: "Delete all conversations?",
      clearAllBody:
        "Every conversation in memory is forgotten and any answer in progress stops.",
      clearAllConfirm: "Delete",
      cleared: "All conversations were deleted.",
      footnote:
        "You use your own key, so usage is billed or limited by the provider. The assistant cannot submit work, change grades or read page and file contents. It asks before changing anything in Moodle, saving a file to your Mac or adding a reminder.",
    },
  },
  errors: {
    /** Shown in French only when a backend message has no translation. */
    general: "Something went wrong.",
    add: "Could not connect this organisation.",
    signIn: "Could not sign in again.",
    settings: "Could not save this setting.",
    remove: "Could not remove this organisation.",
    navigation: "Could not complete this action.",
    assistant: "The assistant could not answer.",
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
    inbox: "Notifications",
    courses: "Cours",
    agenda: "Agenda",
    views: "Vues",
    organisations: "Organisations",
  },
  common: {
    details: "En savoir plus",
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
  sync: {
    title: "Actualisation hors connexion automatique",
    selected: "Choisi pour l’actualisation automatique",
    hint: "Activez-la pour les fichiers choisis dans ce cours. Elle fonctionne uniquement lorsque MoodBuro est ouvert, un fichier à la fois, avec un délai croissant entre les échecs. Les copies restent consultables en cas d’échec. Après annulation ou fermeture, les fichiers partiels redémarrent\u00a0; les fichiers terminés sont conservés. Les suppressions et changements de nom distants ne suppriment aucun fichier local.",
    schedule: "Actualisation",
    paused: "En pause",
    hourly: "Toutes les heures",
    daily: "Chaque jour",
    cancel: "Annuler l’actualisation en cours",
    now: "Actualiser maintenant",
    enable: "Actualiser chaque jour les fichiers choisis",
    replaceSelection: "Utiliser la sélection pour l’actualisation automatique",
    selectionHint:
      "Choisissez 1 à 50 fichiers dans la liste Moodle ci-dessous. Appliquer une nouvelle sélection remplace celle du cours et arrête la tentative en cours. Les fichiers désélectionnés restent sur le Mac. La démonstration enregistre les préférences sans transfert en arrière-plan.",
    count: (total: number, pending: number) =>
      `${total} sélectionnés · ${pending} restants`,
    next: (date: string) =>
      `Prochaine tentative si l’app est ouverte\u00a0: ${date}`,
    missing: (count: number) =>
      `${count} fichiers choisis ne sont plus renvoyés par Moodle. Leur nom, emplacement ou accès a pu changer, ou ils ont été supprimés. Les copies sont conservées. Chargez la liste pour revoir la sélection.`,
    states: {
      paused: "En pause",
      waiting: "En attente d’actualisation",
      running: "Actualisation des fichiers choisis…",
      idle: "Actualisation terminée",
      error: "Échec\u00a0; les fichiers restants seront réessayés",
      cancelled:
        "Tentative annulée\u00a0; la prochaine actualisation reste active",
      interrupted: "Reprise des fichiers restants après redémarrage",
    },
  },
  offline: {
    files: "Fichiers…",
    filesLabel: (name: string) => `Fichiers hors connexion de ${name}`,
    title: "Fichiers du cours",
    back: "Retour aux cours",
    openCourse: "Ouvrir le cours dans Moodle",
    hint: "Enregistrez des fichiers choisis dans les ressources et dossiers Moodle. Les copies s’ouvrent sans connexion. Le chargement et l’actualisation nécessitent une connexion et l’autorisation de télécharger. L’actualisation est à sens unique\u00a0: les fichiers modifiés sont enregistrés à côté des copies existantes, les modifications locales sont conservées et les suppressions Moodle ne suppriment aucun fichier local. Retirer un compte conserve les fichiers dans le dossier du cours du Finder mais efface cette liste. Il ne s’agit pas d’un cours hors connexion complet.",
    remove: "Retirer la copie…",
    confirmRemove: "Retirer la copie",
    removeLabel: (name: string) => `Retirer la copie enregistrée de ${name}`,
    removeTitle: (name: string, site: string) =>
      `Retirer « ${name} » de ${site}\u00a0?`,
    removeBody:
      "Seule cette version quitte la liste hors connexion. Un fichier inchangé est déplacé dans ~/Moodle/.trash, où il reste jusqu’à son retrait dans le Finder. Un fichier modifié localement reste dans son dossier actuel, tout comme un fichier partagé avec une autre entrée ou un téléchargement. Les autres versions et les fichiers Moodle sont conservés. L’espace disque n’est pas libéré immédiatement. Une sélection d’actualisation automatique peut enregistrer à nouveau ce fichier\u00a0; mettez-la en pause ou modifiez-la pour l’éviter.",
    removed: {
      archived:
        "Copie retirée. Le fichier inchangé reste récupérable dans ~/Moodle/.trash (Finder → Aller → Aller au dossier). Aucun nettoyage automatique n’est appliqué.",
      kept: "Entrée retirée. Le fichier a été modifié, est partagé avec une autre entrée ou un téléchargement, ou n’a pas pu être déplacé en toute sécurité ; il reste dans son dossier actuel.",
      missing:
        "Copie indisponible retirée de la liste. Aucun fichier n’a été déplacé.",
    },
    available: "Disponible hors connexion",
    notSaved: "Non enregistré sur ce Mac",
    copyMissing: "Copie enregistrée indisponible",
    lastRefreshed: (date: string) =>
      `Dernière actualisation réussie\u00a0: ${date}`,
    freshnessHint:
      "Actualisez manuellement ou activez une fréquence pour les fichiers choisis. Les fichiers Moodle peuvent avoir changé depuis la dernière actualisation réussie. Les modifications locales et les versions précédentes sont conservées.",
    refreshFailed:
      "Échec de l’actualisation. Cette copie reste consultable ; elle peut différer de Moodle.",
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
    summary:
      "Choisissez quand recevoir un rappel avant les échéances de chaque organisation.",
    help: "Fonctionnement des rappels",
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
  calendar: {
    title: "Abonnement au calendrier",
    scope: "Lecture seule · 5 jours passés, 60 à venir",
    setup: "Configurer…",
    again: "Configurer à nouveau…",
    confirm: "Ouvrir Calendrier",
    connecting: "Vérification…",
    moodle: "Export Moodle…",
    consentTitle: (name: string) => `S’abonner à ${name}\u00a0?`,
    consent:
      "Calendrier conservera un lien privé. Retirer ce compte de MoodBuro ne révoquera pas ce lien.",
    duplicate:
      "Vérifiez d’abord Calendrier pour éviter un abonnement en double.",
    requested: (date: string) => `Configuration demandée le ${date}`,
    sent: "Terminez la configuration et choisissez la fréquence dans Calendrier.",
    details:
      "Retirez les abonnements dans Calendrier. Pour révoquer l’accès, contactez l’administrateur Moodle. Les rappels MoodBuro nécessitent que l’app reste ouverte.",
    demo: "Aperçu uniquement, aucun calendrier ajouté.",
    demoSent: "Aperçu de configuration enregistré.",
  },
  agenda: {
    help: "Export et informations sur l’agenda",
    reminders: "Réglages des rappels…",
    remind: {
      day: (name: string) => `Me le rappeler 24 heures avant\u00a0: ${name}`,
      hour: (name: string) => `Me le rappeler 1 heure avant\u00a0: ${name}`,
      onDay: (name: string) =>
        `Rappel prévu 24 heures avant\u00a0: ${name}. Retirer`,
      onHour: (name: string) =>
        `Rappel prévu 1 heure avant\u00a0: ${name}. Retirer`,
    },
    dates: "Filtrer par dates…",
    clearDates: "Toutes les dates",
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
    timeZone: (zone: string) => `Fuseau horaire\u00a0: ${zone}`,
    footnote:
      "Moodle détermine les évènements à venir affichés ici. L’achèvement et l’éligibilité aux rappels peuvent être incomplets.",
  },
  bookmarks: {
    title: "Signets",
    newTitle: "Nouveau signet",
    editTitle: "Modifier le signet",
    saved: "Signet enregistré.",
    saving: "Enregistrement…",
    removeQuestion: "Supprimer ce signet ?",
    add: "Ajouter cette page aux signets… (⌘D)",
    more: "Autres signets",
    edit: "Modifier le signet",
    remove: "Supprimer le signet",
    name: "Nom",
    icon: "Icône",
    save: "Enregistrer",
    cancel: "Annuler",
    close: "Fermer les signets",
    empty:
      "Ajoutez une page Moodle avec le bouton de signet de la barre d’outils.",
    local: "Enregistrés sur ce Mac pour cette organisation et ce compte.",
    icons: {
      bookmark: "Signet",
      star: "Étoile",
      book: "Livre",
      calendar: "Calendrier",
      file: "Fichier",
      folder: "Dossier",
      message: "Bulle de discussion",
      graduation: "Toque universitaire",
    },
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
    inbox: "Notifications",
    courses: "Cours",
    agenda: "Agenda",
    bookmarks: "Signets",
    quickOpen: "Ouverture rapide (⌘K)",
    sessionWaiting: (time: string) =>
      `Moodle limite la connexion automatique. Nouvel essai à ${time}.`,
    sessionRetrying: "Restauration de votre session Moodle…",
    sessionFailed:
      "Échec de la connexion automatique. Connectez-vous à Moodle ou via les Réglages.",
    sessionHelp:
      "Moodle limite temporairement la connexion automatique après une connexion récente (normalement environ six minutes ; votre site peut imposer un délai plus long). Vos identifiants sont conservés. MoodBuro réessaie une fois à l’heure indiquée tant que l’application reste ouverte. Vous pouvez aussi vous connecter manuellement.",
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
    destination:
      "Enregistrés dans ~/Moodle, par organisation puis par cours. Afficher dans le Finder permet de retrouver l’emplacement exact.",
    folders:
      "Les noms de dossiers contiennent des identifiants internes pour distinguer les organisations et les cours. Sans cours identifié, le fichier va dans le dossier Downloads de l’organisation.",
    feedback: {
      downloading: (name: string) => `Téléchargement de ${name}…`,
      complete: (name: string) => `Téléchargé\u00a0: ${name}`,
      failed: (name: string) => `Échec du téléchargement\u00a0: ${name}`,
    },
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
      "Le compte enregistré de cette organisation et ses identifiants du trousseau seront supprimés de ce Mac. Son profil de navigation privé sera effacé lorsque c’est possible. Sur les versions plus anciennes de macOS, des sessions de connexion partagées peuvent subsister. Votre compte Moodle et vos cours ne sont pas affectés. Retirez les abonnements séparément dans Calendrier.",
    cancel: "Annuler",
    confirmRemove: "Retirer",
    removed: (name: string) => `${name} a été retiré de ce Mac.`,
    removedUnnamed: "Le site a été retiré de ce Mac.",
    reauthenticated: (name: string) => `Connexion à ${name} rétablie.`,
    reauthenticatedUnnamed: "Connexion au site rétablie.",
  },
  assistant: {
    title: "Assistant IA",
    toggle: "Assistant IA (⇧⌘A)",
    general: "Discussion générale",
    scope: (name: string) => `Conversation pour ${name}`,
    close: "Fermer l’assistant",
    clear: "Effacer la conversation…",
    clearTitle: "Effacer cette conversation\u00a0?",
    clearBody:
      "Les messages affichés ici seront oubliés. Les conversations sont conservées en mémoire uniquement et ne sont jamais enregistrées sur le disque.",
    clearConfirm: "Effacer",
    cancel: "Annuler",
    resize: "Largeur du panneau de l’assistant",
    log: "Fil de la conversation",
    you: "Vous",
    answer: "Réponse",
    placeholder: "Posez une question",
    inputLabel: "Message à l’assistant",
    inputHint: "Retour pour envoyer · Maj-Retour pour aller à la ligne",
    send: "Envoyer",
    stop: "Arrêter",
    retry: "Réessayer",
    thinking: "En attente de la réponse…",
    steps: {
      courses: "Consultation de vos cours…",
      activities: "Recherche dans les activités enregistrées…",
      agenda: "Consultation de votre agenda…",
      notifications: "Consultation de vos notifications…",
      files: "Consultation des fichiers du cours…",
      open: "Ouverture dans Moodle…",
      save: "Enregistrement du fichier…",
      confirm: "En attente de votre confirmation…",
    },
    contextLabel: "Contexte Moodle",
    contextNone: "Aucun contexte Moodle",
    contextSite: (name: string) => `Organisation\u00a0: ${name}`,
    contextCourses: "Cours",
    contextNeedsConsent:
      "Le partage du contexte Moodle est désactivé. Activez-le dans les Réglages pour joindre une organisation ou un cours.",
    contextNeedsSite:
      "Ouvrez une organisation pour joindre son contexte Moodle.",
    attached: (name: string) => `Contexte\u00a0: ${name}`,
    sources: "Sources",
    moreSources: (count: number) => `${formatNumber("fr", count)} de plus`,
    fewerSources: "Afficher moins",
    openSource: (label: string) => `Ouvrir dans Moodle\u00a0: ${label}`,
    openFile: (label: string) =>
      `Afficher la copie enregistrée sur ce Mac, ou ouvrir dans Moodle\u00a0: ${label}`,
    suggestions: "Questions suggérées",
    suggest: {
      due: "Qu’ai-je à rendre cette semaine\u00a0?",
      news: "Quoi de neuf dans mes notifications\u00a0?",
      files: "Quels fichiers contient ce cours\u00a0?",
      courses: "Quels sont mes cours\u00a0?",
    },
    setupTitle: "Configurer l’assistant",
    setupHint:
      "Ajoutez votre propre clé Albert API ou Mistral AI dans les Réglages pour démarrer une conversation.",
    openSettings: "Ouvrir les Réglages…",
    emptyTitle: "Posez une question",
    emptyHint: (provider: string) =>
      `Vos messages sont envoyés à ${provider}. La conversation reste en mémoire jusqu’à ce que vous quittiez MoodBuro et n’est jamais enregistrée sur le disque.`,
    confirm: {
      "mark-read": {
        title: () => "Marquer cette notification comme lue\u00a0?",
        body: (site: string) =>
          `Dans ${site}. Rien ne change dans Moodle sans votre confirmation.`,
        approve: "Marquer comme lue",
        decline: "Ne pas marquer",
      },
      "save-file": {
        title: () => "Enregistrer ce fichier sur votre Mac\u00a0?",
        body: (site: string, size: string | null) =>
          `Depuis ${site}, dans le dossier du cours de votre dossier Moodle${size ? ` (${size})` : ""}. Rien ne change dans Moodle.`,
        approve: "Enregistrer",
        decline: "Ne pas enregistrer",
      },
      reminder: {
        title: (when: string | null) =>
          when === "hour"
            ? "Vous rappeler cette échéance 1 heure avant\u00a0?"
            : "Vous rappeler cette échéance 24 heures avant\u00a0?",
        body: (site: string) =>
          `Dans ${site}. Une notification sur ce Mac, envoyée uniquement si MoodBuro est en cours d’exécution. Vous pouvez la retirer dans l’agenda.`,
        approve: "Ajouter le rappel",
        decline: "Ne pas ajouter",
      },
    },
    notes: {
      courses: "Liste de vos cours consultée",
      activities: "Recherche dans les activités enregistrées sur ce Mac",
      agenda: "Agenda consulté",
      notifications:
        "Objets des notifications enregistrés sur ce Mac consultés",
      files: "Noms des fichiers du cours consultés",
      saved: (label: string) =>
        `Enregistré dans votre dossier Moodle\u00a0: ${label}`,
      saveFailed: (label: string) =>
        `Impossible d’enregistrer «\u00a0${label}\u00a0».`,
      saveDeclined: (label: string) => `Non enregistré\u00a0: ${label}`,
      reminderDay: (label: string) =>
        `Rappel ajouté 24 heures avant\u00a0: ${label}`,
      reminderHour: (label: string) =>
        `Rappel ajouté 1 heure avant\u00a0: ${label}`,
      reminderFailed: (label: string) =>
        `Impossible d’ajouter un rappel pour «\u00a0${label}\u00a0».`,
      reminderDeclined: (label: string) =>
        `Aucun rappel ajouté\u00a0: ${label}`,
      updated: (when: string) => `actualisé ${when}`,
      updatedNow: "actualisé à l’instant",
      stale:
        "actualisation impossible, les données enregistrées ont été utilisées",
      opened: (label: string) => `Ouvert dans Moodle\u00a0: ${label}`,
      openedNotifications: "Page des notifications ouverte dans Moodle",
      openedCalendar: "Calendrier ouvert dans Moodle",
      marked: (label: string) => `Marquée comme lue\u00a0: ${label}`,
      declined: (label: string) => `Laissée non lue\u00a0: ${label}`,
      writeFailed: (label: string) =>
        `Moodle n’a pas accepté de marquer «\u00a0${label}\u00a0» comme lue.`,
      refused: "Une action demandée n’est pas disponible et a été ignorée.",
      stopped: "Arrêté avant la fin de la réponse.",
    },
    settings: {
      title: "Assistant IA",
      intro:
        "Facultatif. Discutez dans un panneau latéral avec votre propre clé Albert API ou Mistral AI.",
      provider: "Fournisseur",
      providerHint:
        "Chaque fournisseur conserve sa propre clé et son modèle. Changer de fournisseur efface les conversations.",
      key: "Clé d’API",
      keySaved: "Une clé est enregistrée dans le trousseau de votre Mac.",
      keyMissing: "Aucune clé enregistrée pour l’instant.",
      keyPlaceholder: "Collez votre clé d’API",
      saveKey: "Enregistrer la clé",
      removeKey: "Supprimer la clé…",
      removeKeyTitle: (provider: string) =>
        `Supprimer la clé ${provider}\u00a0?`,
      removeKeyBody:
        "La clé est supprimée du trousseau de votre Mac et toute réponse en cours s’arrête. La clé elle-même reste valide chez le fournisseur.",
      removeKeyConfirm: "Supprimer",
      model: "Modèle",
      modelHint:
        "Vérifiez la connexion pour choisir parmi les modèles accessibles avec votre clé.",
      modelLabel: "Nom du modèle",
      saveModel: "Enregistrer",
      check: "Vérifier la connexion",
      checking: "Vérification…",
      connected: (count: number) =>
        `Connexion établie. ${formatNumber("fr", count)} ${plural("fr", count) === "one" ? "modèle de discussion disponible" : "modèles de discussion disponibles"}.`,
      consent: "Partager le contexte Moodle",
      consentHint: (provider: string) =>
        `Désactiver le partage arrête la réponse en cours et efface les conversations. Désactivé\u00a0: l’assistant ne voit que ce que vous saisissez. Activé\u00a0: lorsque vous joignez une organisation ou un cours à un message, MoodBuro envoie aussi à ${provider} le nom de l’organisation ainsi que les noms de cours, les noms d’activités enregistrées, les noms des fichiers de cours avec leur section et leur taille, les échéances à venir et les objets de notifications que l’assistant consulte. Les jetons de connexion, les cookies, les clés d’API, votre nom, les adresses des sites et le contenu des fichiers ne sont jamais envoyés.`,
      history: "Conversations",
      historyHint:
        "Conservées en mémoire uniquement, séparément pour chaque compte d’organisation, jusqu’à ce que vous quittiez MoodBuro. Rien n’est écrit sur le disque.",
      clearAll: "Supprimer toutes les conversations…",
      clearAllTitle: "Supprimer toutes les conversations\u00a0?",
      clearAllBody:
        "Toutes les conversations en mémoire sont oubliées et toute réponse en cours s’arrête.",
      clearAllConfirm: "Supprimer",
      cleared: "Toutes les conversations ont été supprimées.",
      footnote:
        "Vous utilisez votre propre clé\u00a0: l’usage est facturé ou limité par le fournisseur. L’assistant ne peut ni rendre un devoir, ni modifier une note, ni lire le contenu des pages et des fichiers. Il demande votre accord avant toute modification dans Moodle, tout enregistrement de fichier sur votre Mac et tout ajout de rappel.",
    },
  },
  errors: {
    general: "Une erreur inattendue s’est produite.",
    add: "Impossible de connecter cette organisation.",
    signIn: "Impossible de vous reconnecter.",
    settings: "Impossible d’enregistrer ce réglage.",
    remove: "Impossible de retirer cette organisation.",
    navigation: "Impossible d’effectuer cette action.",
    assistant: "L’assistant n’a pas pu répondre.",
    detail: "Détail\u00a0:",
  },
};

export const catalogues: Record<Locale, Messages> = { en, fr };
