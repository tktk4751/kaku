use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;

/// ソケットファイルのパスを取得
fn get_socket_path() -> PathBuf {
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR")
        .unwrap_or_else(|_| "/tmp".to_string());
    PathBuf::from(runtime_dir).join("kaku.sock")
}

/// 既存インスタンスにコマンドを送信
pub fn send_command(command: &str) -> Result<String, String> {
    let socket_path = get_socket_path();

    let mut stream = UnixStream::connect(&socket_path)
        .map_err(|e| format!("Failed to connect to socket: {}", e))?;

    writeln!(stream, "{}", command)
        .map_err(|e| format!("Failed to send command: {}", e))?;

    let mut reader = BufReader::new(stream);
    let mut response = String::new();
    reader.read_line(&mut response)
        .map_err(|e| format!("Failed to read response: {}", e))?;

    Ok(response.trim().to_string())
}

/// 既存インスタンスが存在するか確認
pub fn is_instance_running() -> bool {
    send_command("ping").is_ok()
}

/// IPCサーバーを起動
pub fn start_ipc_server<F>(on_toggle: F) -> Result<(), String>
where
    F: Fn() + Send + Sync + 'static,
{
    let socket_path = get_socket_path();

    // 古いソケットファイルを削除
    let _ = fs::remove_file(&socket_path);

    let listener = UnixListener::bind(&socket_path)
        .map_err(|e| format!("Failed to bind socket: {}", e))?;

    println!("[IPC] Server listening on {:?}", socket_path);

    let on_toggle = Arc::new(on_toggle);

    thread::spawn(move || {
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let on_toggle = Arc::clone(&on_toggle);
                    thread::spawn(move || {
                        handle_client(stream, &*on_toggle);
                    });
                }
                Err(e) => {
                    eprintln!("[IPC] Connection error: {}", e);
                }
            }
        }
    });

    Ok(())
}

fn handle_client<F>(stream: UnixStream, on_toggle: &F)
where
    F: Fn(),
{
    let mut reader = BufReader::new(&stream);
    let mut writer = &stream;

    let mut line = String::new();
    if reader.read_line(&mut line).is_ok() {
        let command = line.trim();
        println!("[IPC] Received command: {}", command);

        let response = match command {
            "ping" => "pong".to_string(),
            "toggle" => {
                on_toggle();
                "toggled".to_string()
            }
            "show" => {
                on_toggle(); // TODO: implement show-only
                "shown".to_string()
            }
            _ => "unknown command".to_string(),
        };

        let _ = writeln!(writer, "{}", response);
    }
}

/// ソケットファイルを削除（終了時）
pub fn cleanup() {
    let socket_path = get_socket_path();
    let _ = fs::remove_file(&socket_path);
}
