use std::path::PathBuf;
use crate::{errors::LauncherError, helpers, injector, samp};
use log::{error, info, warn};
use md5::compute;
use sevenz_rust::decompress_file;
use std::fs::File;
use std::io::{Read, Write};

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

fn irx_voice_archive_complete(archive_path: &str, gtasa_path: &str) -> std::result::Result<bool, String> {
    let archive_file = File::open(archive_path)
        .map_err(|e| format!("Could not open iRxVoice ZIP: {}", e))?;
    let game_dir = PathBuf::from(gtasa_path);
    if !game_dir.join("gta_sa.exe").is_file() {
        return Err("GTA: San Andreas executable was not found".to_string());
    }

    let mut archive = zip::ZipArchive::new(archive_file)
        .map_err(|e| format!("Invalid iRxVoice ZIP archive: {}", e))?;
    let mut found_plugin = false;

    for index in 0..archive.len() {
        let entry = archive.by_index(index)
            .map_err(|e| format!("Could not read ZIP entry: {}", e))?;
        if entry.is_dir() {
            continue;
        }

        let relative_path = entry.enclosed_name()
            .ok_or_else(|| "The ZIP contains an unsafe file path".to_string())?;
        if entry.unix_mode().map(|mode| mode & 0o170000 == 0o120000).unwrap_or(false) {
            return Err("The ZIP contains an unsupported symbolic link".to_string());
        }

        if relative_path.file_name()
            .and_then(|name| name.to_str())
            .map(|name| name.eq_ignore_ascii_case("iRxVoice.asi"))
            .unwrap_or(false) {
            found_plugin = true;
        }

        let installed_path = game_dir.join(relative_path);
        let metadata = match std::fs::metadata(&installed_path) {
            Ok(metadata) if metadata.is_file() => metadata,
            _ => return Ok(false),
        };
        if metadata.len() != entry.size() {
            return Ok(false);
        }
    }

    Ok(found_plugin)
}

fn irx_voice_archive_has_files(archive_path: &str, gtasa_path: &str) -> std::result::Result<bool, String> {
    let archive_file = File::open(archive_path)
        .map_err(|e| format!("Could not open iRxVoice ZIP: {}", e))?;
    let game_dir = PathBuf::from(gtasa_path);
    let mut archive = zip::ZipArchive::new(archive_file)
        .map_err(|e| format!("Invalid iRxVoice ZIP archive: {}", e))?;

    for index in 0..archive.len() {
        let entry = archive.by_index(index)
            .map_err(|e| format!("Could not read ZIP entry: {}", e))?;
        if entry.is_dir() {
            continue;
        }
        let relative_path = entry.enclosed_name()
            .ok_or_else(|| "The ZIP contains an unsafe file path".to_string())?;
        if entry.unix_mode().map(|mode| mode & 0o170000 == 0o120000).unwrap_or(false) {
            return Err("The ZIP contains an unsupported symbolic link".to_string());
        }
        if game_dir.join(relative_path).is_file() {
            return Ok(true);
        }
    }

    Ok(false)
}

#[tauri::command]
pub fn has_irx_voice_archive_files(archive_path: String, gtasa_path: String) -> std::result::Result<bool, String> {
    #[cfg(windows)]
    {
        irx_voice_archive_has_files(&archive_path, &gtasa_path)
    }
    #[cfg(not(windows))]
    {
        let _ = (archive_path, gtasa_path);
        Err("iRxVoice installation is supported on Windows only".to_string())
    }
}

#[tauri::command]
pub fn is_irx_voice_archive_installed(archive_path: String, gtasa_path: String) -> std::result::Result<bool, String> {
    #[cfg(windows)]
    {
        irx_voice_archive_complete(&archive_path, &gtasa_path)
    }
    #[cfg(not(windows))]
    {
        let _ = (archive_path, gtasa_path);
        Err("iRxVoice installation is supported on Windows only".to_string())
    }
}

#[tauri::command]
pub fn install_irx_voice_archive(archive_path: String, gtasa_path: String) -> std::result::Result<(), String> {
    #[cfg(windows)]
    {
        let archive_file = File::open(&archive_path)
            .map_err(|e| format!("Could not open iRxVoice ZIP: {}", e))?;
        let game_dir = PathBuf::from(&gtasa_path);
        if !game_dir.join("gta_sa.exe").is_file() {
            return Err("GTA: San Andreas executable was not found".to_string());
        }

        let mut archive = zip::ZipArchive::new(archive_file)
            .map_err(|e| format!("Invalid iRxVoice ZIP archive: {}", e))?;
        let mut found_plugin = false;

        for index in 0..archive.len() {
            let entry = archive.by_index(index)
                .map_err(|e| format!("Could not read ZIP entry: {}", e))?;
            if entry.is_dir() {
                continue;
            }
            let relative_path = entry.enclosed_name()
                .ok_or_else(|| "The ZIP contains an unsafe file path".to_string())?;
            if entry.unix_mode().map(|mode| mode & 0o170000 == 0o120000).unwrap_or(false) {
                return Err("The ZIP contains an unsupported symbolic link".to_string());
            }
            if relative_path.file_name()
                .and_then(|name| name.to_str())
                .map(|name| name.eq_ignore_ascii_case("iRxVoice.asi"))
                .unwrap_or(false) {
                found_plugin = true;
            }
        }

        if !found_plugin {
            return Err("iRxVoice.asi was not found inside the ZIP archive".to_string());
        }

        for index in 0..archive.len() {
            let mut entry = archive.by_index(index)
                .map_err(|e| format!("Could not read ZIP entry: {}", e))?;
            let relative_path = entry.enclosed_name()
                .ok_or_else(|| "The ZIP contains an unsafe file path".to_string())?;
            if entry.is_dir() {
                std::fs::create_dir_all(game_dir.join(relative_path))
                    .map_err(|e| format!("Could not create GTA folder: {}", e))?;
                continue;
            }

            if entry.unix_mode().map(|mode| mode & 0o170000 == 0o120000).unwrap_or(false) {
                return Err("The ZIP contains an unsupported symbolic link".to_string());
            }

            let destination = game_dir.join(relative_path);
            let parent = destination.parent()
                .ok_or_else(|| "Invalid destination path in ZIP".to_string())?;
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Could not create destination folder: {}", e))?;

            let mut output = File::create(&destination)
                .map_err(|e| format!("Could not replace '{}': {}", destination.display(), e))?;
            std::io::copy(&mut entry, &mut output)
                .map_err(|e| format!("Could not extract '{}': {}", destination.display(), e))?;
            output.flush()
                .map_err(|e| format!("Could not finish writing '{}': {}", destination.display(), e))?;
        }

        if !irx_voice_archive_complete(&archive_path, &gtasa_path)? {
            return Err("Not all iRxVoice archive files were installed successfully".to_string());
        }
        Ok(())
    }
    #[cfg(not(windows))]
    {
        let _ = (archive_path, gtasa_path);
        Err("iRxVoice installation is supported on Windows only".to_string())
    }
}
