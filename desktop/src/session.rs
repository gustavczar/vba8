use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Default)]
pub struct Session {
    pub selected: Mutex<Option<PathBuf>>,
    busy: Arc<AtomicBool>,
}

pub struct OperationGuard(Arc<AtomicBool>);
impl Drop for OperationGuard {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

impl Session {
    pub fn enter(&self) -> Result<OperationGuard, String> {
        self.busy
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| "Outra operação está em andamento. Aguarde sua conclusão.".to_string())?;
        Ok(OperationGuard(Arc::clone(&self.busy)))
    }
}

pub fn selected_source(session: &Session, requested: &str) -> Result<PathBuf, String> {
    let selected = session
        .selected
        .lock()
        .map_err(|_| "Sessão indisponível. Reinicie o aplicativo.".to_string())?;
    match selected.as_ref() {
        Some(path) if path == Path::new(requested) => Ok(path.clone()),
        _ => Err("Selecione a planilha pelo botão antes de gerar uma cópia.".into()),
    }
}

pub fn suggested_output(source: &Path) -> String {
    let stem = source
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("planilha");
    format!("{stem}_recuperada.xlsm")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn operation_guard_blocks_concurrent_invocations_and_releases_on_drop() {
        let session = Session::default();
        let first = session.enter().expect("primeira operação deve entrar");
        assert!(session.enter().is_err());
        drop(first);
        assert!(session.enter().is_ok());
    }

    #[test]
    fn recovery_requires_a_native_selection_and_exact_selected_path() {
        let session = Session::default();
        assert!(selected_source(&session, "C:/sample.xlsm").is_err());
        *session.selected.lock().unwrap() = Some(PathBuf::from("C:/sample.xlsm"));
        assert_eq!(
            selected_source(&session, "C:/sample.xlsm").unwrap(),
            PathBuf::from("C:/sample.xlsm")
        );
        assert!(selected_source(&session, "C:/other.xlsm").is_err());
    }

    #[test]
    fn output_name_preserves_unicode_and_is_not_the_original() {
        assert_eq!(
            suggested_output(Path::new("C:/Planilhas/Ação mensal.xlsm")),
            "Ação mensal_recuperada.xlsm"
        );
    }
}
