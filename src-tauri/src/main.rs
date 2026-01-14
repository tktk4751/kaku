// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();

    // --toggle オプションの処理
    if args.iter().any(|arg| arg == "--toggle") {
        if kaku_lib::platform::is_instance_running() {
            match kaku_lib::platform::send_command("toggle") {
                Ok(response) => {
                    println!("Toggle response: {}", response);
                    return;
                }
                Err(e) => {
                    eprintln!("Failed to send toggle command: {}", e);
                    // インスタンスが動いていない場合は通常起動
                }
            }
        } else {
            println!("No running instance found, starting new instance...");
        }
    }

    // --show オプションの処理
    if args.iter().any(|arg| arg == "--show") {
        if kaku_lib::platform::is_instance_running() {
            match kaku_lib::platform::send_command("show") {
                Ok(response) => {
                    println!("Show response: {}", response);
                    return;
                }
                Err(e) => {
                    eprintln!("Failed to send show command: {}", e);
                }
            }
        }
    }

    // 通常起動
    kaku_lib::run()
}
