use std::{env, path::Path, process::ExitCode};
fn main() -> ExitCode {
    let args: Vec<_> = env::args_os().collect();
    let result = match args.get(1).and_then(|s| s.to_str()) {
        Some("--inspect") if args.len() == 3 => vba8_core::inspect(Path::new(&args[2]))
            .and_then(|r| serde_json::to_value(r).map_err(|e| vba8_core::Error(e.to_string()))),
        Some("--recover") if args.len() == 4 => {
            vba8_core::recover(Path::new(&args[2]), Path::new(&args[3]))
                .and_then(|r| serde_json::to_value(r).map_err(|e| vba8_core::Error(e.to_string())))
        }
        _ => Err(vba8_core::Error(
            "Uso: vba8-core --inspect arquivo.xlsm | --recover origem.xlsm copia.xlsm".into(),
        )),
    };
    match result {
        Ok(value) => {
            println!("{value}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{}", serde_json::json!({"error":e.to_string()}));
            ExitCode::FAILURE
        }
    }
}
