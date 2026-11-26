//! French texts (translation catalog).

use super::Text;

pub fn text(text: Text) -> &'static str {
    match text {
        Text::ResultApplication => "Application",
        Text::ResultStoreApp => "Application du Store",
        Text::ResultOpenInBrowser => "Ouvrir dans le navigateur",
        Text::ResultSearchWith => "Rechercher avec {engine}",

        Text::CommandSettingsTitle => "Paramètres L8",
        Text::CommandSettingsSubtitle => "Raccourci, widgets et apparence",
        Text::CommandSettingsKeywords => "parametres preferences configuration options raccourci",
        Text::CommandReindexTitle => "Reconstruire l'index de recherche",
        Text::CommandReindexSubtitle => "Analyser à nouveau les applications et fichiers",
        Text::CommandReindexKeywords => "actualiser reindexer analyser mettre a jour",
        Text::CommandQuitTitle => "Quitter L8 Toolkit",
        Text::CommandQuitSubtitle => "Ferme tous les widgets",
        Text::CommandQuitKeywords => "quitter fermer arreter",
        Text::CommandLockTitle => "Verrouiller l'écran",
        Text::CommandLockSubtitle => "Verrouille la session Windows",
        Text::CommandLockKeywords => "verrouiller session absent",
        Text::CommandWindowsSettingsTitle => "Paramètres Windows",
        Text::CommandWindowsSettingsSubtitle => "Paramètres système",
        Text::CommandWindowsSettingsKeywords => "parametres panneau de configuration systeme",
        Text::CommandRecycleBinTitle => "Corbeille",
        Text::CommandRecycleBinSubtitle => "Fichiers supprimés",
        Text::CommandRecycleBinKeywords => "corbeille supprimes poubelle",

        Text::OutcomeFileCopied => "Copié, collez-le dans n'importe quel dossier",
        Text::OutcomeMovedToRecycleBin => "Déplacé dans la Corbeille",
        Text::OutcomePinned => "Épinglé dans Fréquents",
        Text::OutcomeUnpinned => "Désépinglé",
        Text::OutcomeRemovedFromFrequent => "Retiré des Fréquents",
        Text::OutcomeRebuildingIndex => "Reconstruction de l'index",
        Text::OutcomeCopied => "Copié",

        Text::ErrorOnlyProgramsElevate => "Seuls les programmes peuvent s'exécuter en administrateur",
        Text::ErrorOpenWithFilesOnly => "« Ouvrir avec » n'est disponible que pour les fichiers",
        Text::ErrorOnlyFilesDeletable => "Seuls les fichiers et dossiers peuvent être supprimés",
        Text::ErrorOnlyPinnable => "Seuls les applications, fichiers et dossiers peuvent être épinglés",
        Text::ErrorNoFileOnDisk => "{title} n'a pas de fichier sur le disque",
        Text::ErrorNothingPlaying => "Aucune lecture en cours",
        Text::ErrorPlayerRefused => "Le lecteur n'a pas accepté cette action",
        Text::ErrorPlayerUnknown => "L'application du lecteur est inconnue",
        Text::ErrorMediaUnavailable => "Les commandes multimédias ne sont pas disponibles",
        Text::ErrorShortcutInUse => "{shortcut} est déjà utilisé par une autre application",
        Text::ErrorShortcutInvalid => "{shortcut} n'est pas un raccourci valide",

        Text::TrayOpenSearch => "Ouvrir la recherche",
        Text::TraySettings => "Paramètres…",
        Text::TrayInstallUpdate => "Installer la mise à jour {version}…",
        Text::TrayQuit => "Quitter L8 Toolkit",

        Text::UpdateCheckFailed => "Impossible de rechercher les mises à jour. Vérifiez votre connexion et réessayez.",
        Text::UpdateInstallFailed => "La mise à jour n'a pas pu être installée. Réessayez dans un instant.",
        Text::UpdateBusy => "Une recherche de mise à jour est déjà en cours",
        Text::UpdateNotificationTitle => "L8 Toolkit {version} est disponible",
        Text::UpdateNotificationBody => "Installez-la en un clic depuis Paramètres › Mises à jour, ou depuis l'icône de la zone de notification.",

        Text::TimerDoneTitle => "Temps écoulé",
        Text::TimerDoneBody => "Votre minuteur est terminé.",
        Text::PomodoroFocusDoneTitle => "Session de concentration terminée",
        Text::PomodoroBreakDoneTitle => "La pause est finie",
        Text::PomodoroFocusStarted => "On s'y remet : {minutes} minutes de concentration ont commencé.",
        Text::PomodoroFocusReady => "Lancez la prochaine session de {minutes} minutes depuis l'encoche.",
        Text::PomodoroBreakStarted => "Faites une pause de {minutes} minutes, elle a commencé.",
        Text::PomodoroBreakReady => "Lancez votre pause de {minutes} minutes depuis l'encoche.",
    }
}
