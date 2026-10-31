#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod session;

use serde::Serialize;
use serde_json::Value;
use session::{selected_source, suggested_output, Session};
use tauri_plugin_dialog::DialogExt;

#[derive(Serialize)]
struct Selection {
    path: String,
    inspection: Value,
}

#[derive(Serialize)]
struct Recovery {
    output: String,
    report: Value,
}

#[tauri::command]
async fn select_source(
    app: tauri::AppHandle,
    state: tauri::State<'_, Session>,
) -> Result<Option<Selection>, String> {
    let _guard = state.enter()?;
    *state.selected.lock().map_err(|_| "Sessão indisponível")? = None;
    let result = tauri::async_runtime::spawn_blocking(move || {
        let choice = app
            .dialog()
            .file()
            .set_title("Selecionar planilha XLSM — VBA8")
            .add_filter("Planilha Excel com macros", &["xlsm"])
            .blocking_pick_file();
        let Some(choice) = choice else {
            return Ok(None);
        };
        let path = choice
            .as_path()
            .ok_or("Escolha um arquivo local.")?
            .to_path_buf();
        let inspection = vba8_core::inspect(&path).map_err(|error| error.to_string())?;
        let inspection = serde_json::to_value(inspection).map_err(|error| error.to_string())?;
        Ok::<_, String>(Some((path, inspection)))
    })
    .await
    .map_err(|_| "Não foi possível concluir a inspeção.".to_string())??;
    match result {
        None => Ok(None),
        Some((path, inspection)) => {
            *state.selected.lock().map_err(|_| "Sessão indisponível")? = Some(path.clone());
            Ok(Some(Selection {
                path: path.to_string_lossy().into_owned(),
                inspection,
            }))
        }
    }
}

#[tauri::command]
async fn recover_selected(
    app: tauri::AppHandle,
    state: tauri::State<'_, Session>,
    source: String,
) -> Result<Option<Recovery>, String> {
    let _guard = state.enter()?;
    let src = selected_source(&state, &source)?;
    tauri::async_runtime::spawn_blocking(move || {
        // Re-inspection and all writing safeguards are in the core, not the UI.
        vba8_core::inspect(&src).map_err(|error| error.to_string())?;
        let choice = app
            .dialog()
            .file()
            .set_title("Salvar NOVA cópia XLSM — VBA8")
            .set_file_name(suggested_output(&src))
            .set_directory(src.parent().unwrap_or_else(|| std::path::Path::new(".")))
            .add_filter("Planilha Excel com macros", &["xlsm"])
            .blocking_save_file();
        let Some(choice) = choice else {
            return Ok(None);
        };
        let dst = choice
            .as_path()
            .ok_or("Escolha um destino local.")?
            .to_path_buf();
        let report = vba8_core::recover(&src, &dst).map_err(|error| error.to_string())?;
        Ok::<_, String>(Some(Recovery {
            output: dst.to_string_lossy().into_owned(),
            report: serde_json::to_value(report).map_err(|error| error.to_string())?,
        }))
    })
    .await
    .map_err(|_| "Não foi possível concluir a gravação.".to_string())?
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(Session::default())
        .invoke_handler(tauri::generate_handler![select_source, recover_selected])
        .run(tauri::generate_context!())
        .expect("Falha ao iniciar VBA8. Verifique o runtime Microsoft Edge WebView2.");
}
