use std::net::{SocketAddr, UdpSocket};
use std::str;

fn main() -> std::io::Result<()> {
    let args: Vec<String> = std::env::args().collect();

    let mode = args.get(1).map(|s| s.as_str()).unwrap_or("");
    match mode {
        "send" => {
            let host = match args.get(2) {
                Some(v) => v,
                None => {
                    eprintln!("Usage: {} send <host> <port>", args[0]);
                    return Ok(());
                }
            };

            let port_str = match args.get(3) {
                Some(v) => v,
                None => {
                    eprintln!("Usage: {} send <host> <port>", args[0]);
                    return Ok(());
                }
            };

            let port: u16 = match port_str.parse() {
                Ok(p) => p,
                Err(_) => {
                    eprintln!("Invalid port: {}", port_str);
                    return Ok(());
                }
            };

            udp_send(host, port)?;
        }
        "receive" => {
            let port_str = match args.get(2) {
                Some(v) => v,
                None => {
                    eprintln!("Usage: {} receive <port>", args[0]);
                    return Ok(());
                }
            };

            let port: u16 = match port_str.parse() {
                Ok(p) => p,
                Err(_) => {
                    eprintln!("Invalid port: {}", port_str);
                    return Ok(());
                }
            };

            udp_receive(port)?;
        }
        _ => {
            eprintln!("Usage: {} [send|receive]", args[0]);
        }
    }

    Ok(())
}

fn udp_send(host: &str, port: u16) -> std::io::Result<()> {
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    let addr: SocketAddr = format!("{}:{}", host, port)
        .parse()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;

    let message = "Hello, UDP!";
    socket.send_to(message.as_bytes(), addr)?;
    println!("Sent '{}' to {}", message, addr);
    Ok(())
}

fn udp_receive(port: u16) -> std::io::Result<()> {
    let socket = UdpSocket::bind(("0.0.0.0", port))?;
    println!("Listening on port {}...", port);

    let mut buffer = [0u8; 1024];

    loop {
        let (bytes_read, sender_addr) = socket.recv_from(&mut buffer)?;
        match str::from_utf8(&buffer[..bytes_read]) {
            Ok(received) => {
                println!(
                    "Received {} bytes from {}: {}",
                    bytes_read, sender_addr, received
                );
            }
            Err(_) => {
                println!(
                    "Received {} bytes from {} (non-UTF8 data)",
                    bytes_read, sender_addr
                );
            }
        }
    }
}
