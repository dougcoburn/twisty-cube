mod commands;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::get_solved,
            commands::apply_moves,
            commands::validate_facelets,
            commands::scramble,
            commands::solve_twophase
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
