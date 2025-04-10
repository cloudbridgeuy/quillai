mod cli;
mod parser;
mod prelude;

use clap::Parser;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};

#[allow(unused_imports)]
use tokio::net::{TcpListener, TcpStream};

use crate::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    // ╭─────────────────────────────────────────────────────────────────────────────╮
    // │ App                                                                         │
    // ╰─────────────────────────────────────────────────────────────────────────────╯
    let args = crate::cli::App::parse();

    // ╭─────────────────────────────────────────────────────────────────────────────╮
    // │ Logger                                                                      │
    // ╰─────────────────────────────────────────────────────────────────────────────╯
    env_logger::builder()
        .filter_level(log::LevelFilter::from(args.log_level))
        .init();

    // ╭─────────────────────────────────────────────────────────────────────────────╮
    // │ Start listening                                                             │
    // ╰─────────────────────────────────────────────────────────────────────────────╯
    listen(args.bind).await
}

#[allow(dead_code)]
async fn listen(addr: std::net::SocketAddr) -> Result<()> {
    let listener = TcpListener::bind(addr).await?;
    log::trace!("SMTP server listening on {}", addr);

    loop {
        match listener.accept().await {
            #[allow(unused_variables)]
            #[allow(unused_mut)]
            Ok((mut stream, _)) => {
                tokio::spawn(async move {
                    if let Err(err) = handle_connection(&mut stream).await {
                        log::trace!("Error handling SMTP connection: {:?}", err);
                    };
                });
            }
            Err(err) => {
                log::error!("Error establishing SMTP connection: {:?}", err);
            }
        }
    }
}

async fn handle_connection(stream: &mut TcpStream) -> Result<()> {
    let (reader, writer) = stream.split();
    let mut reader = BufReader::new(reader);
    let mut writer = BufWriter::new(writer);

    writer
        .write_all(format!("220 {}\r\n", "My SMTP Server").as_bytes())
        .await?;
    writer.flush().await?;

    let mut line = String::new();

    loop {
        line.clear();
        let bytes_read = reader.read_line(&mut line).await?;
        log::trace!("Read {} bytes: {:?}", bytes_read, line);

        if bytes_read == 0 {
            break;
        }

        // Parse the line
        // React to the parsed line

        writer
            .write_all(format!("Read {} bytes", bytes_read).as_bytes())
            .await?;
        writer.flush().await?;
    }

    Ok(())

    // loop {
    //     line.clear();
    //     let bytes_read = reader.lines
    //         (&mut line).await?;
    //     if bytes_read == 0 {
    //         break;
    //     }
    //
    //     let response = match line.trim() {
    //         "HELO" => "250 Hello\r\n",
    //         "QUIT" => {
    //             writer.write_all(b"221 Bye\r\n").await?;
    //             break;
    //         }
    //         _ => "500 Unrecognized command\r\n",
    //     };
    //
    //     writer.write_all(response.as_bytes()).await?;
    //     writer.flush().await?;
    // }
}
