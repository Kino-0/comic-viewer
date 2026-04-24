use std::fs;
use std::path::Path;
use std::process::Command;

#[tauri::command]
async fn search_with_plocate(query: &str, db_path: &str) -> Result<Vec<String>, String> {
    if query.is_empty() {
        return Ok(vec![]);
    }

    let output = Command::new("plocate")
        .arg("-d")
        .arg(db_path)
        .arg("-i")
        .arg(query)
        .output()
        .map_err(|e| e.to_string())?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    let results: Vec<String> = stdout
        .lines()
        .filter(|line| Path::new(line).is_dir())
        .map(|s| s.to_string())
        .collect();

    Ok(results)
}

#[tauri::command]
async fn get_images_in_dir(path: String) -> Result<Vec<String>, String> {
    let supported_extensions = ["png", "jpg", "jpeg", "webp", "gif", "bmp", "tiff"];

    let mut image_paths = Vec::new();

    // ディレクトリの内容を読み込む
    let entries = match fs::read_dir(&path) {
        Ok(e) => e,
        Err(e) => return Err(format!("ディレクトリの読み込みに失敗しました: {}", e)),
    };

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue, // エラーが発生したエントリはスキップ
        };

        let path_buf = entry.path();

        // ファイルか確認し、拡張子をチェック
        if path_buf.is_file() {
            if let Some(extension) = path_buf.extension().and_then(|s| s.to_str()) {
                let lower_extension = extension.to_lowercase();
                if supported_extensions.contains(&lower_extension.as_str()) {
                    // パス全体をStringとして格納
                    image_paths.push(path_buf.to_string_lossy().to_string());
                }
            }
        }
    }

    Ok(image_paths)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            search_with_plocate,
            get_images_in_dir
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
