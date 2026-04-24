use std::path::PathBuf;

use aerotrans_lib::desktop_registration::{
    build_app_paths_commands, build_shortcut_script, start_menu_shortcut_path,
};

#[test]
fn app_paths_commands_target_hkcu_for_current_user_registration() {
    let exe_path = PathBuf::from(r"C:\Apps\AeroTrans\AeroTrans.exe");

    let commands = build_app_paths_commands(&exe_path).unwrap();

    assert_eq!(commands.len(), 2);
    assert!(commands[0].contains(
        r"HKCU\Software\Microsoft\Windows\CurrentVersion\App Paths\AeroTrans.exe"
    ));
    assert!(commands[0].contains(r"C:\Apps\AeroTrans\AeroTrans.exe"));
    assert!(commands[1].contains(r"/v Path"));
    assert!(commands[1].contains(r"C:\Apps\AeroTrans"));
}

#[test]
fn start_menu_shortcut_path_uses_current_user_programs_folder() {
    let shortcut_path = start_menu_shortcut_path(&PathBuf::from(r"C:\Users\Alice\AppData\Roaming"));

    assert_eq!(
        shortcut_path,
        PathBuf::from(
            r"C:\Users\Alice\AppData\Roaming\Microsoft\Windows\Start Menu\Programs\AeroTrans.lnk"
        )
    );
}

#[test]
fn shortcut_script_points_to_exe_and_shortcut_location() {
    let shortcut_path = PathBuf::from(
        r"C:\Users\Alice\AppData\Roaming\Microsoft\Windows\Start Menu\Programs\AeroTrans.lnk",
    );
    let exe_path = PathBuf::from(r"C:\Apps\AeroTrans\AeroTrans.exe");

    let script = build_shortcut_script(&shortcut_path, &exe_path).unwrap();

    assert!(script.contains("CreateShortcut"));
    assert!(script.contains(r"C:\Users\Alice\AppData\Roaming\Microsoft\Windows\Start Menu\Programs\AeroTrans.lnk"));
    assert!(script.contains(r"C:\Apps\AeroTrans\AeroTrans.exe"));
    assert!(script.contains("AeroTrans desktop translator"));
}
