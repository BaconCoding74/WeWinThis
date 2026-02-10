mod gcs;

fn main() -> std::io::Result<()> {
    let args: Vec<String> = std::env::args().collect();

    let mode = args.get(1).map(|s| s.as_str()).unwrap_or("");
    match mode {
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

            gcs::run_gcs(port)?;
        }
        _ => {
            eprintln!("Usage: {} receive <port>", args[0]);
        }
    }

    Ok(())
}
