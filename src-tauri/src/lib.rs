//! Tauriアプリケーションのバックエンドロジックとコマンドを提供するモジュール。

mod db;
use db::Info;
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
};
use tauri::Manager;
use tauri_plugin_dialog::{DialogExt, MessageDialogKind};
use walkdir::WalkDir;

/// 指定されたディレクトリ直下にある画像ファイルパスのリストを取得します。
///
/// * 非再帰的: サブディレクトリ内のファイルは検索されません。
/// * 拡張子: サポートしている拡張子は `png`, `jpg`, `jpeg`, `webp`, `gif` です。
/// * Case: 拡張子の判定において、大文字と小文字は区別されません（例: `.PNG` や `.Jpg` も対象となります）。
/// * エンコーディング: ファイルパスや拡張子が有効なUTF-8文字列に変換できないファイルは、結果からスキップされます。
/// * アクセス権限: 個別のファイルエントリの読み込みに失敗した場合は、そのファイルをスキップします。
///
/// # Arguments
///
/// * `path` - 画像を検索する対象のディレクトリパス。
///
/// # Returns
///
/// 成功した場合は、画像パスのリスト (`Vec<String>`) を返します。
///
/// # Errors
///
/// 以下の理由などにより、指定されたディレクトリ自体の読み込みに失敗した場合はエラーメッセージ（`String`）を返します。
/// * ディレクトリが存在しない
/// * ディレクトリに対する読み取り権限がない
#[tauri::command]
fn get_images_in_dir(path: std::path::PathBuf) -> Result<Vec<String>, String> {
    const SUPPORTED_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "webp", "gif"];

    let entries = std::fs::read_dir(&path)
        .map_err(|e| format!("ディレクトリの読み込みに失敗しました: {}", e))?;

    let image_paths: Vec<String> = entries
        .filter_map(Result::ok) // エラーになったエントリはスキップ
        .filter(|entry| entry.file_type().is_ok_and(|ft| ft.is_file())) // シンボリックリンク等を除外
        .filter_map(|entry| {
            let path = entry.path();
            let extension = path.extension()?.to_str()?; // 拡張子がない、またはUTF-8でない場合はスキップ

            if SUPPORTED_EXTENSIONS
                .iter()
                .any(|&s| s.eq_ignore_ascii_case(extension))
            // 大文字・小文字を区別しない
            {
                Some(path.to_str()?.to_owned()) // パスがUTF-8でない場合はスキップ
            } else {
                None
            }
        })
        .collect();

    Ok(image_paths)
}

/// `info.txt` のテキスト内容を解析し、[`Info`] 構造体に変換します。
///
/// コロン `:` で区切られたキーと値のペアを読み取り、適切なフィールドに割り当てます。
///
/// # Arguments
///
/// * `content` - `info.txt` の文字列データ。
///
/// # Returns
///
/// パースされたギャラリー情報を保持する [`Info`] 構造体。
fn parse_info_txt(content: &str) -> Info {
    let mut info = Info::default();

    // カンマ区切りの文字列を分割するクロージャ
    let parse_csv = |val: &str| -> Vec<String> {
        val.split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    };

    for line in content.lines() {
        let line = line.trim();

        if line.is_empty() {
            continue;
        }

        let Some((key, value)) = line.split_once(':') else {
            continue;
        };

        let key = key.trim();
        let value = value.trim();

        if value == "N/A" {
            continue;
        }

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

    info
}

/// 単一のギャラリー情報（`info.txt`）を読み込み、データベースにインポートします。
///
/// ファイルの内容を解析してデータベースに保存します。対象の `info.txt` に
/// ギャラリーIDが存在しない場合、データベースで採番された新しいIDを
/// 元のファイルに追記します。
///
/// # Arguments
///
/// * `path` - インポートする `info.txt` ファイルのパス。
/// * `db` - ギャラリー情報を保存するデータベースインスタンスへの参照。
///
/// # Returns
///
/// ファイルの読み書きやデータベースの挿入処理でエラーが発生した場合は、エラー文字列を返します。
fn import_single_gallery(path: &Path, db: &db::Database) -> Result<(), String> {
    let content = fs::read_to_string(path).map_err(|e| format!("ファイル読み込みエラー: {}", e))?;

    let info = parse_info_txt(&content);

    let dir_path = path
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();

    // DBへの挿入
    let item_id = db
        .insert_info(&info, &dir_path)
        .map_err(|e| format!("DB挿入エラー: {}", e))?;

    // ギャラリーIDが新規採番された場合、info.txtに追記
    if info.gallery_id.is_none() {
        let mut file = OpenOptions::new()
            .append(true)
            .open(path)
            .map_err(|e| format!("info.txtオープンエラー: {}", e))?;

        writeln!(file, "ギャラリーID: {}", item_id)
            .map_err(|e| format!("info.txt追記エラー: {}", e))?;
    }

    Ok(())
}

/// 指定されたパスを再帰的にスキャンし、見つかった `info.txt` をデータベースにインポートします。
///
/// # Arguments
///
/// * `path` - スキャンを開始するディレクトリのパス。
/// * `db` - アプリケーションのステートとして管理されているデータベースインスタンス。
///
/// # Returns
///
/// 正常にインポートされたギャラリーの総数を返します。
#[tauri::command]
fn scan_and_import(path: String, db: tauri::State<'_, db::Database>) -> Result<usize, String> {
    let mut imported_count = 0;

    for entry in WalkDir::new(&path).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_file() && entry.file_name() == "info.txt" {
            match import_single_gallery(entry.path(), &db) {
                Ok(_) => imported_count += 1,
                Err(e) => eprintln!("インポート失敗 ({:?}): {}", entry.path(), e),
            }
        }
    }

    Ok(imported_count) // 成功したインポート件数を返す
}

/// クエリ文字列に基づいてデータベースを検索し、一致するアイテムのパスを取得します。
///
/// # Arguments
///
/// * `query` - 検索クエリ文字列。
/// * `db` - アプリケーションのステートとして管理されているデータベースインスタンス。
///
/// # Returns
///
/// 検索にヒットしたパスのリストを返し、データベースエラーが発生した場合はエラー文字列を返します。
#[tauri::command]
fn search_items(query: String, db: tauri::State<'_, db::Database>) -> Result<Vec<String>, String> {
    db.search_items(&query).map_err(|e| e.to_string())
}

/// Tauriアプリケーションを初期化し、実行します。
///
/// データベースの接続設定、各種プラグインの初期化、およびTauriコマンドの登録を行います。
/// データベースの初期化に失敗した場合、エラーダイアログを表示してアプリケーションを終了します。
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            // アプリケーション固有のデータディレクトリを取得する
            let mut db_path = app
                .path()
                .app_local_data_dir()
                .expect("Failed to get local data dir");

            // ディレクトリが存在しない場合は作成する
            fs::create_dir_all(&db_path).expect("Failed to create local data dir");

            // データベースのファイル名をパスに追加する
            db_path.push("comic_viewer.db");
            match db::Database::new(&db_path) {
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
