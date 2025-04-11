mod cli;
mod parser;
mod prelude;

use clap::Parser;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};

#[allow(unused_imports)]
use tokio::net::{TcpListener, TcpStream};

use crate::parser::SmtpCommand;
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
    Init,
    Command(SmtpCommand),
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

    let mut state = SmtpState::Init;

    let mut line = String::new();
    let mut mail_from = String::new();
    let mut rcpt_to = String::new();

    loop {
        line.clear();
        let bytes_read = reader.read_line(&mut line).await?;
        if bytes_read == 0 {
            break;
        }

        log::debug!("Received line: {}", line.trim());

        // Move to the next state
        state = match state {
            SmtpState::Init => {
                let command = match crate::parser::parse_ehlo(line.as_str()) {
                    Ok((_, c)) => c,
                    Err(e) => {
                        log::error!("Failed to parse EHLO command: {:?}", e);
                        writer.write_all(b"500 Unrecognized command\r\n").await?;
                        writer.flush().await?;
                        break;
                    }
                };
                SmtpState::Command(command)
            }
            SmtpState::Command(_) => {
                let command = match crate::parser::parse_command(line.as_str()) {
                    Ok((_, c)) => c,
                    Err(e) => {
                        log::error!("Failed to parse command: {:?}", e);
                        writer.write_all(b"500 Unrecognized command\r\n").await?;
                        writer.flush().await?;
                        break;
                    }
                };

                SmtpState::Command(command)
            }
            SmtpState::Data => todo!(),
            SmtpState::Quit => {
                writer.write_all(b"221 Bye\r\n").await?;
                break;
            }
        };

        match state {
            SmtpState::Init => {
                log::error!("State never moved out of initialization. Dropping connection.");
                writer.write_all(b"500 Internal error\r\n").await?;
                writer.flush().await?;
                break;
            }
            SmtpState::Command(ref command) => {
                log::info!("Parsed command: {:?}", command);
                // Handle the command here
                match command {
                    SmtpCommand::Ehlo => {
                        writer.write_all(b"250 Hello\r\n").await?;
                    }
                    SmtpCommand::MailFrom(email) => {
                        mail_from = email.clone();
                        writer.write_all(b"250 OK\r\n").await?;
                    }
                    SmtpCommand::RcptTo(email) => {
                        rcpt_to = email.clone();
                        writer.write_all(b"250 OK\r\n").await?;
                    }
                    SmtpCommand::Data => {
                        writer
                            .write_all(b"354 Start mail input; end with <CRLF>.<CRLF>\r\n")
                            .await?;
                    }
                    SmtpCommand::Quit => {
                        writer.write_all(b"221 Bye\r\n").await?;
                        break;
                    }
                }
                writer.flush().await?;
            }
            _ => {}
        }
    }

    log::debug!("Mail from {} to {}", mail_from, rcpt_to);
    Ok(())
}
