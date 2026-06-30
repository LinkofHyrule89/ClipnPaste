use std::io::Write;
use std::os::unix::net::UnixStream;

fn main() {
    let cmd = std::env::args().nth(1).unwrap_or_else(|| {
        eprintln!("usage: clipnpaste-cli <clipboard|snip>");
        std::process::exit(1);
    });

    if cmd != "clipboard" && cmd != "snip" {
        eprintln!("unknown command: {cmd}");
        std::process::exit(1);
    }

    let socket = dirs::cache_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
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