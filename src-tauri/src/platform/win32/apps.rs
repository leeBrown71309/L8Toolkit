use std::path::Path;

use windows::core::Interface;
use windows::Win32::Storage::EnhancedStorage::PKEY_Link_TargetParsingPath;
use windows::Win32::UI::Shell::{
    BHID_EnumItems, FOLDERID_AppsFolder, IEnumShellItems, IShellItem, IShellItem2, SHGetKnownFolderItem,
    KF_FLAG_DEFAULT, SIGDN_NORMALDISPLAY, SIGDN_PARENTRELATIVEPARSING,
};

use super::com::{enter_com, take_com_string};
use crate::platform::InstalledApp;

const APPS_FOLDER_PREFIX: &str = "shell:AppsFolder\\";

/// Lists every launchable app through the virtual `AppsFolder`.
///
/// This is the same source as the Start menu "All apps" list, so it covers
/// classic desktop programs and Store (UWP/MSIX) apps with a single code path.
pub fn enumerate_installed_apps() -> Result<Vec<InstalledApp>, String> {
    let _apartment = enter_com();

    // SAFETY: standard shell enumeration on an STA thread; every returned
    // string is released by `take_com_string`.
    unsafe {
        let folder: IShellItem = SHGetKnownFolderItem(&FOLDERID_AppsFolder, KF_FLAG_DEFAULT, None)
            .map_err(|error| format!("Cannot open the Apps folder: {error}"))?;
        let items: IEnumShellItems = folder
            .BindToHandler(None, &BHID_EnumItems)
            .map_err(|error| format!("Cannot enumerate the Apps folder: {error}"))?;

        let mut apps = Vec::new();
        loop {
            let mut batch: [Option<IShellItem>; 1] = [None];
            let mut fetched = 0u32;
            let status = items.Next(&mut batch, Some(&mut fetched));
            if status.is_err() || fetched == 0 {
                break;
            }
            let Some(item) = batch[0].take() else { continue };

            let name = item.GetDisplayName(SIGDN_NORMALDISPLAY).map(take_com_string);
            let app_id = item.GetDisplayName(SIGDN_PARENTRELATIVEPARSING).map(take_com_string);
            if let (Ok(name), Ok(app_id)) = (name, app_id) {
                if !name.is_empty() && !app_id.is_empty() {
                    let location = executable_location(&item, &app_id);
                    apps.push(InstalledApp { name, launch_target: format!("{APPS_FOLDER_PREFIX}{app_id}"), location });
                }
            }
        }
        Ok(apps)
    }
}

/// Desktop apps expose their shortcut target; Store apps have no executable
/// the user could reveal or inspect, so they return `None`.
unsafe fn executable_location(item: &IShellItem, app_id: &str) -> Option<String> {
    let from_shortcut = item
        .cast::<IShellItem2>()
        .ok()
        .and_then(|item| item.GetString(&PKEY_Link_TargetParsingPath).ok())
        .map(take_com_string)
        .filter(|path| !path.is_empty());
    let candidate = from_shortcut.or_else(|| Path::new(app_id).is_absolute().then(|| app_id.to_string()))?;
    Path::new(&candidate).exists().then_some(candidate)
}
