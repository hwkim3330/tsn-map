use std::process::Command;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            // Try to start the backend server
            let exe_dir = std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|p| p.to_path_buf()));

            if let Some(dir) = exe_dir {
                let backend_path = dir.join("tsn-map");
                if backend_path.exists() {
                    log::info!("Starting backend from: {:?}", backend_path);
                    let _ = Command::new(&backend_path)
                        .arg("-p")
                        .arg("8080")
                        .spawn();

                    // Wait for server to start
                    std::thread::sleep(std::time::Duration::from_millis(1000));
                }
            }

            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
