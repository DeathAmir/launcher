use std::path::PathBuf;
use std::process::Command;
use crate::{errors::LauncherError, helpers, injector, samp};
use log::{error, info, warn};
use md5::compute;
use sevenz_rust::decompress_file;
use std::fs::File;
use std::io::Read;

#[tauri::command]
pub async fn inject(
    name: &str,
    ip: &str,
    port: i32,
    exe: &str,
    dll: &str,
    omp_file: &str,
    password: &str,
    custom_game_exe: &str,
) -> std::result::Result<(), String> {
    let actual_omp_file = if *crate::NO_OMP_FLAG.lock().unwrap() {
        ""
    } else {
        omp_file
    };

    crate::ipc::ensure_listening();

    match injector::run_samp(
        name,
        ip,
        port,
        exe,
        dll,
        actual_omp_file,
        password,
        custom_game_exe,
    )
    .await
    {
        Ok(_) => Ok(()),
        Err(e) => {
            log::warn!("{}", e);
            match e {
                LauncherError::AccessDenied(_) => {
                    return Err("need_admin".to_string());
                }
                _ => return Err(e.to_string()),
            }
        }
    }
}

#[tauri::command]
pub fn get_gtasa_path_from_samp() -> String {
    samp::get_gtasa_path()
}

#[tauri::command]
pub fn get_model_cache_folder() -> std::result::Result<String, String> {
    samp::get_model_cache_folder()
}

#[tauri::command]
pub fn set_model_cache_folder(folder: String) -> std::result::Result<String, String> {
    samp::set_model_cache_folder(folder)
}

#[tauri::command]
pub fn get_nickname_from_samp() -> String {
    samp::get_nickname()
}

#[tauri::command]
pub fn rerun_as_admin() -> std::result::Result<String, String> {
    let exe_path = std::env::current_exe()
        .map_err(|_| "Failed to get current executable path".to_string())?
        .into_os_string()
        .into_string()
        .map_err(|_| "Failed to convert path to string".to_string())?;

    runas::Command::new(exe_path)
        .arg("")
        .status()
        .map_err(|_| "Failed to restart as administrator".to_string())?;

    Ok("SUCCESS".to_string())
}

#[tauri::command]
pub fn get_samp_favorite_list() -> String {
    samp::get_samp_favorite_list()
}

#[tauri::command]
pub fn resolve_hostname(hostname: String) -> std::result::Result<String, String> {
    helpers::resolve_hostname_to_ipv4(&hostname)
}

#[tauri::command]
pub fn is_wine() -> bool {
    #[cfg(windows)]
    return crate::wine::is_wine();
    #[cfg(not(windows))]
    false
}

#[tauri::command]
pub fn is_process_alive(pid: u32) -> bool {
    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
    use windows_sys::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};

    unsafe {
        let handle: HANDLE = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle != 0 {
            CloseHandle(handle);
            true
        } else {
            false
        }
    }
}

#[tauri::command]
pub fn get_checksum_of_files(list: Vec<String>) -> std::result::Result<Vec<String>, String> {
    let mut result = Vec::new();

    for file in list {
        let mut f =
            File::open(&file).map_err(|e| format!("Failed to open file '{}': {}", file, e))?;

        let mut contents = Vec::new();
        f.read_to_end(&mut contents)
            .map_err(|e| format!("Failed to read file '{}': {}", file, e))?;

        let digest = compute(&contents);
        let checksum_entry = format!("{}|{:x}", file, digest);
        result.push(checksum_entry);
    }

    Ok(result)
}

#[tauri::command]
pub fn extract_7z(path: String, output_path: String) -> std::result::Result<(), String> {
    decompress_file(&path, &output_path)
        .map_err(|e| format!("Failed to extract archive '{}': {}", path, e))
}

#[tauri::command]
pub fn copy_files_to_gtasa(src: String, gtasa_dir: String) -> std::result::Result<(), String> {
    match helpers::copy_files(&src, &gtasa_dir) {
        Ok(_) => Ok(()),
        Err(e) => {
            log::warn!("{}", e);
            match e {
                LauncherError::AccessDenied(_) => {
                    return Err("need_admin".to_string());
                }
                _ => return Err(e.to_string()),
            }
        }
    }
}

#[tauri::command]
pub fn log_info(msg: &str) -> () {
    info!("Frontend info: {}", msg);
}

#[tauri::command]
pub fn log_warn(msg: &str) -> () {
    warn!("Frontend warning: {}", msg);
}

#[tauri::command]
pub fn log_error(msg: &str) -> () {
    error!("Frontend error: {}", msg);
}

#[tauri::command]
pub fn install_irx_voice_archive(archive_path: String, gtasa_path: String) -> std::result::Result<(), String> {
    #[cfg(windows)]
    {
        let archive = PathBuf::from(&archive_path);
        let game_dir = PathBuf::from(&gtasa_path);
        if !archive.is_file() { return Err("iRxVoice archive was not downloaded".to_string()); }
        if !game_dir.join("gta_sa.exe").is_file() { return Err("GTA: San Andreas executable was not found".to_string()); }
        let extract_dir = std::env::temp_dir().join(format!("irxvoice-{}", std::process::id()));
        if extract_dir.exists() { std::fs::remove_dir_all(&extract_dir).map_err(|e| e.to_string())?; }
        std::fs::create_dir_all(&extract_dir).map_err(|e| e.to_string())?;
        let extract_path = extract_dir.to_string_lossy().to_string();
        let script = "$ErrorActionPreference = 'Stop'; Expand-Archive -LiteralPath $args[0] -DestinationPath $args[1] -Force; $plugin = Get-ChildItem -LiteralPath $args[1] -Filter 'iRxVoice.asi' -File -Recurse | Select-Object -First 1; if ($null -eq $plugin) { exit 3 }; Copy-Item -LiteralPath $plugin.FullName -Destination (Join-Path $args[2] 'iRxVoice.asi') -Force";
        let status = Command::new("powershell.exe").args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command", script, &archive_path, &extract_path, &gtasa_path]).status().map_err(|e| format!("Could not start ZIP extraction: {}", e))?;
        let _ = std::fs::remove_dir_all(&extract_dir);
        if !status.success() { return Err("Could not extract iRxVoice.asi from the downloaded ZIP".to_string()); }
        if !game_dir.join("iRxVoice.asi").is_file() { return Err("iRxVoice.asi installation could not be verified".to_string()); }
        Ok(())
    }
    #[cfg(not(windows))]
    { let _ = (archive_path, gtasa_path); Err("iRxVoice installation is supported on Windows only".to_string()) }
}
