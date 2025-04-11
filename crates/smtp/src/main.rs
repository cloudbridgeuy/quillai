mod cli;
mod parser;
mod prelude;

use clap::Parser;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};

#[allow(unused_imports)]
use tokio::net::{TcpListener, TcpStream};

use crate::parser::{Header, SmtpCommand};
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

#[derive(Clone, Debug, PartialEq)]
pub enum SmtpState {
    Command,
    Data,
    Quit,
}

async fn handle_connection(stream: &mut TcpStream) -> Result<()> {
    let (reader, writer) = stream.split();
    let mut reader = BufReader::new(reader);
    let mut writer = BufWriter::new(writer);

    writer
        .write_all(format!("220 {}\r\n", "My SMTP Server").as_bytes())
        .await?;
    writer.flush().await?;

    let mut state = SmtpState::Command;

    let mut line = String::new();

    let mut mailfrom: Option<String> = None;
    let mut rcpts: Vec<String> = Vec::new();
    let mut headers: Vec<Header> = Vec::new();
    let mut parsing_headers = true;
    let mut message = String::new();

    loop {
        line.clear();
        let bytes_read = reader.read_line(&mut line).await?;
        if bytes_read == 0 {
            break;
        }
        log::debug!("Received line: {}", line.trim());

        // Move to the next state
        match state {
            SmtpState::Command => {
                let command = match crate::parser::parse_command(line.as_str()) {
                    Ok((_, c)) => c,
                    Err(e) => {
                        log::error!("Failed to parse command: {:?}", e);
                        writer.write_all(b"500 Unrecognized command\r\n").await?;
                        writer.flush().await?;
                        break;
                    }
                };

                match command {
                    SmtpCommand::Ehlo => {
                        writer.write_all(b"250 Hello\r\n").await?;
                        writer.flush().await?;
                    }
                    SmtpCommand::MailFrom(email) => {
                        mailfrom = Some(email.clone());
                        writer.write_all(b"250 OK\r\n").await?;
                        writer.flush().await?;
                    }
                    SmtpCommand::RcptTo(email) => {
                        rcpts.push(email.clone());
                        writer.write_all(b"250 OK\r\n").await?;
                        writer.flush().await?;
                    }
                    SmtpCommand::Noop => {
                        writer.write_all(b"250 OK\r\n").await?;
                        writer.flush().await?;
                    }
                    SmtpCommand::Rset => {
                        mailfrom = None;
                        rcpts = Vec::new();
                        headers = Vec::new();
                        message = String::new();
                        writer.write_all(b"250 OK\r\n").await?;
                        writer.flush().await?;
                    }
                    SmtpCommand::Data => {
                        writer
                            .write_all(b"354 Start mail input; end with <CRLF>.<CRLF>\r\n")
                            .await?;
                        writer.flush().await?;
                        state = SmtpState::Data;
                    }
                    SmtpCommand::Quit => {
                        writer.write_all(b"221 Bye\r\n").await?;
                        writer.flush().await?;
                    }
                }
            }
            SmtpState::Data => {
                if line.trim() == "." {
                    writer.write_all(b"250 OK\r\n").await?;
                    writer.flush().await?;
                    state = SmtpState::Quit;
                } else if parsing_headers && line == "\r\n" {
                    parsing_headers = false;
                } else if parsing_headers {
                    match crate::parser::parse_header(line.as_str()) {
                        Ok((_, h)) => {
                            headers.push(h);
                        }
                        Err(e) => {
                            log::error!("Failed to parse header: {:?}", e);
                            writer.write_all(b"500 Unrecognized header\r\n").await?;
                            writer.flush().await?;
                            break;
                        }
                    };
                } else {
                    message.push_str(&line);
                    message.push('\n');
                }
            }
            SmtpState::Quit => {
                let command = match crate::parser::parse_command(line.as_str()) {
                    Ok((_, c)) => c,
                    Err(e) => {
                        log::error!("Failed to parse command: {:?}", e);
                        writer.write_all(b"500 Unrecognized command\r\n").await?;
                        writer.flush().await?;
                        break;
                    }
                };

                match command {
                    SmtpCommand::Quit => {
                        writer.write_all(b"221 Bye\r\n").await?;
                        writer.flush().await?;
                    }
                    _ => {
                        writer.write_all(b"500 Unrecognized command\r\n").await?;
                        writer.flush().await?;
                    }
                };
            }
        };
    }

    log::debug!(
        "Mail from {} to {}",
        mailfrom.unwrap_or_default(),
        rcpts.join(", ")
    );
    log::debug!("Headers: {:?}", headers);
    log::debug!("Message: {:?}", message);
    Ok(())
}
