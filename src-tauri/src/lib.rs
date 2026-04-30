mod db;
use db::Info;
use std::fs;
use tauri::Manager;
use tauri_plugin_dialog::{DialogExt, MessageDialogKind};
use walkdir::WalkDir;

#[tauri::command]
async fn get_images_in_dir(path: String) -> Result<Vec<String>, String> {
    let supported_extensions = ["png", "jpg", "jpeg", "webp", "gif"];

    let mut image_paths = Vec::new();

    // ディレクトリの内容を読み込む
    let entries =
        fs::read_dir(&path).map_err(|e| format!("ディレクトリの読み込みに失敗しました: {}", e))?;

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

// info.txt の内容をパースするヘルパー関数
fn parse_info_txt(content: &str) -> Info {
    let mut info = Info::default();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        if let Some((key, value)) = line.split_once(':') {
            let key = key.trim();
            let value = value.trim();

            if value == "N/A" {
                continue;
            }

            // カンマ区切りの文字列を分割するクロージャ
            let parse_csv = |val: &str| -> Vec<String> {
                val.split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            };

            match key {
                "ギャラリーID" => info.gallery_id = value.parse::<i64>().ok(),
                "タイトル" => info.title = value.to_string(),
                "作者" => info.artists = parse_csv(value),
                "グループ" => info.groups = parse_csv(value),
                "種類" => info.type_name = value.to_string(),
                "シリーズ" => info.series = parse_csv(value),
                "キャラクター" => info.characters = parse_csv(value),
                "タグ" => info.tags = parse_csv(value),
                "言語" => info.language = value.to_string(),
                _ => {}
            }
        }
    }

    info
}

#[tauri::command]
async fn scan_and_import(
    path: String,
    db: tauri::State<'_, db::Database>,
) -> Result<usize, String> {
    let mut imported_count = 0;

    // walkdirを使ってinfo.txtを探す
    for entry in WalkDir::new(&path).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_file() && entry.file_name() == "info.txt" {
            let path_buf = entry.path();

            let content = match fs::read_to_string(path_buf) {
                Ok(c) => c,
                Err(_) => continue, // 読み込みエラーはスキップ
            };

            let info = parse_info_txt(&content);
            let dir_path = path_buf
                .parent()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();

            // DBへの挿入
            if let Err(e) = db.insert_info(&info, &dir_path) {
                eprintln!("DB挿入エラー ({:?}): {}", path_buf, e);
            } else {
                imported_count += 1;
            }
        }
    }

    Ok(imported_count) // 成功したインポート件数を返す
}

#[tauri::command]
async fn search_items(
    query: String,
    db: tauri::State<'_, db::Database>,
) -> Result<Vec<String>, String> {
    db.search_items(&query).map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            match db::Database::new("comic_viewer.db") {
                Ok(db) => {
                    app.manage(db);
                }
                Err(e) => {
                    let err_msg = format!("データベースの初期化に失敗しました: {}", e);
                    eprintln!("{}", err_msg);
                    // エラーメッセージダイアログを表示する
                    app.dialog()
                        .message(&err_msg)
                        .title("初期化エラー")
                        .kind(MessageDialogKind::Error)
                        .show(|_| {
                            std::process::exit(1);
                        });
                }
            };
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_images_in_dir,
            scan_and_import,
            search_items
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
