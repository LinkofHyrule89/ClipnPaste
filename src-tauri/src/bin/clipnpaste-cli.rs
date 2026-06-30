use std::io::Write;
use std::os::unix::net::UnixStream;

fn main() {
    let cmd = std::env::args().nth(1).unwrap_or_else(|| {
        eprintln!("usage: clipnpaste-cli <clipboard|emoji|snip>");
        std::process::exit(1);
    });

    if cmd != "clipboard" && cmd != "emoji" && cmd != "snip" {
        eprintln!("unknown command: {cmd}");
        std::process::exit(1);
    }

    if cmd == "clipboard" || cmd == "emoji" {
        clipnpaste_lib::focus_target::capture_to_file();
    }

    let socket = dirs::data_local_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("clipnpaste")
        .join("ipc.sock");

    let mut stream = match UnixStream::connect(&socket) {
        Ok(stream) => stream,
        Err(err) => {
            eprintln!("ClipnPaste is not running ({err})");
            std::process::exit(1);
        }
    };

    if let Err(err) = write!(stream, "{cmd}") {
        eprintln!("failed to send command: {err}");
        std::process::exit(1);
    }
}