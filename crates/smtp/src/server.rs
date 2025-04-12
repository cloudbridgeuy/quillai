use std::io::Read;

use futures::{stream::iter, SinkExt, StreamExt};
use native_tls::{Identity, TlsAcceptor};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::net::{TcpListener, TcpStream};
use tokio_native_tls::{TlsAcceptor as TokioTlsAcceptor, TlsStream};
use tokio_util::codec::{Framed, LinesCodec};

use crate::parser::{Header, SmtpCommand};
use crate::prelude::*;

#[derive(Clone, Debug, PartialEq)]
pub enum SmtpState {
    Command,
    Data,
    Quit,
}

#[derive(Clone)]
pub struct Server {
    pub cert_data: Vec<u8>,
    pub key_data: Vec<u8>,
}

pub struct Connection {
    server: Server,
}

impl Server {
    pub fn new(cert_file_path: String, key_file_path: String) -> Result<Self> {
        // Read the certificate file
        let mut cert_file = std::fs::File::open(cert_file_path)?;
        let mut cert_data = Vec::new();
        cert_file.read_to_end(&mut cert_data)?;
        // let pem_certificate: &[u8] = &cert_data;

        // Read the private key file
        let mut key_file = std::fs::File::open(key_file_path)?;
        let mut key_data = Vec::new();
        key_file.read_to_end(&mut key_data)?;
        // let pem_private_key: &[u8] = &key_data;

        Ok(Self {
            cert_data,
            key_data,
        })
    }

    pub async fn listen(self, addr: std::net::SocketAddr) -> Result<()> {
        let listener = TcpListener::bind(addr).await?;
        log::trace!("SMTP server listening on {}", addr);

        loop {
            match listener.accept().await {
                #[allow(unused_variables)]
                #[allow(unused_mut)]
                Ok((mut stream, _)) => {
                    let connection = Connection::new(self.clone());
                    tokio::spawn(async move {
                        if let Err(err) = connection.handle(&mut stream).await {
                            // if let Err(err) = handle_connection(&mut stream).await {
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
}

impl Connection {
    pub fn new(server: Server) -> Self {
        Self { server }
    }

    async fn send_commands(
        framed: &mut Framed<TlsStream<&mut TcpStream>, LinesCodec>,
        commands: Vec<String>,
    ) -> Result<()> {
        // Only need to add \r because the codec only adds \n
        let messages = iter(commands.into_iter().map(|x| format!("{}\r", x)));
        framed
            .send_all(&mut messages.map(|m| {
                log::debug!("<- {}", m.trim());
                Ok(m)
            }))
            .await?;
        Ok(())
    }

    pub async fn handle(self, stream: &mut TcpStream) -> Result<()> {
        let (reader, writer) = stream.split();
        let mut writer = BufWriter::new(writer);
        let mut reader = BufReader::new(reader);

        let mut is_tls = false;
        let mut line = String::new();

        writer.write_all(b"220 My SMTP server\r\n").await?;
        writer.flush().await?;

        while reader.read_line(&mut line).await? != 0 {
            let command = match crate::parser::parse_command(line.as_str()) {
                Ok((_, c)) => c,
                Err(e) => {
                    log::error!("Failed to parse command: {:?}", e);
                    writer.write_all(b"500 Unknown command\r\n").await?;
                    writer.flush().await?;
                    break;
                }
            };

            match command {
                SmtpCommand::Ehlo => {
                    writer.write_all(b"250-windmill Hello\r\n").await?;
                    writer.write_all(b"250-STARTTLS\r\n").await?;
                    writer.write_all(b"250 What you've got?\r\n").await?;
                    writer.flush().await?;
                }
                SmtpCommand::Starttls => {
                    writer.write_all(b"220 Ready to start TLS\r\n").await?;
                    writer.flush().await?;
                    is_tls = true;
                    break;
                }
                SmtpCommand::Quit => {
                    writer.write_all(b"221 Bye\r\n").await?;
                    writer.flush().await?;
                    return Ok(());
                }
                SmtpCommand::Noop => {
                    writer.write_all(b"250 Ok\r\n").await?;
                    writer.flush().await?;
                }
                SmtpCommand::MailFrom(_) | SmtpCommand::RcptTo(_) => {
                    writer
                        .write_all(b"530 Must issue a STARTTLS command first\r\n")
                        .await?;
                    writer.flush().await?;
                }
                SmtpCommand::Data | SmtpCommand::Rset => {
                    writer
                        .write_all(b"530 Must issue a STARTTLS command first\r\n")
                        .await?;
                    writer.flush().await?;
                }
            }

            line.clear();
        }

        if is_tls {
            self.handle_starttls(stream).await
        } else {
            Err(anyhow::anyhow!("TLS not established"))
        }
    }

    pub async fn handle_starttls(self, stream: &mut TcpStream) -> Result<()> {
        let pem_certificate: &[u8] = &self.server.cert_data;
        let pem_private_key: &[u8] = &self.server.key_data;

        let identity = Identity::from_pkcs8(pem_certificate, pem_private_key)?;
        let tls_acceptor = TlsAcceptor::builder(identity).build()?;
        let tls_acceptor = TokioTlsAcceptor::from(tls_acceptor);

        match tls_acceptor.accept(stream).await {
            Ok(stream) => self.handle_session(stream).await,
            Err(e) => {
                log::error!("Error establishing SMTP TLS connection: {:?}", e);
                Err(e.into())
            }
        }
    }

    pub async fn handle_session(self, stream: TlsStream<&mut TcpStream>) -> Result<()> {
        let mut state = SmtpState::Command;

        let mut mailfrom: Option<String> = None;
        let mut rcpts: Vec<String> = Vec::new();
        let mut headers: Vec<Header> = Vec::new();
        let mut parsing_headers = true;
        let mut message = String::new();

        let mut framed = Framed::new(stream, LinesCodec::new());

        // Connection::send_commands(&mut framed, vec![format!("220 {}", "My SMTP Server")]).await?;

        while let Some(line_str) = framed.next().await {
            let line = line_str?;
            log::debug!("-> {}", line.trim());

            // Move to the next state
            match state {
                SmtpState::Command => {
                    let command = match crate::parser::parse_command(line.as_str()) {
                        Ok((_, c)) => c,
                        Err(e) => {
                            log::error!("Failed to parse command: {:?}", e);
                            Connection::send_commands(
                                &mut framed,
                                vec!["500 Unrecognized command".to_string()],
                            )
                            .await?;
                            break;
                        }
                    };

                    match command {
                        SmtpCommand::Ehlo => {
                            Connection::send_commands(&mut framed, vec!["250 Hello".to_string()])
                                .await?;
                        }
                        SmtpCommand::MailFrom(email) => {
                            mailfrom = Some(email.clone());
                            Connection::send_commands(&mut framed, vec!["250 Ok".to_string()])
                                .await?;
                        }
                        SmtpCommand::RcptTo(email) => {
                            rcpts.push(email.clone());
                            Connection::send_commands(&mut framed, vec!["250 Ok".to_string()])
                                .await?;
                        }
                        SmtpCommand::Noop => {
                            Connection::send_commands(&mut framed, vec!["250 Ok".to_string()])
                                .await?;
                        }
                        SmtpCommand::Rset => {
                            mailfrom = None;
                            rcpts = Vec::new();
                            headers = Vec::new();
                            message = String::new();
                            Connection::send_commands(&mut framed, vec!["250 Ok".to_string()])
                                .await?;
                        }
                        SmtpCommand::Data => {
                            Connection::send_commands(
                            &mut framed,
                            vec!["354 Start mail input; end with <CRLF>.<CRLF>\r\n".to_string()],
                        )
                        .await?;
                            state = SmtpState::Data;
                        }
                        SmtpCommand::Quit => {
                            Connection::send_commands(&mut framed, vec!["221 Bye".to_string()])
                                .await?;
                        }
                        _ => {
                            Connection::send_commands(
                                &mut framed,
                                vec!["500 Unrecognized command".to_string()],
                            )
                            .await?;
                            break;
                        }
                    }
                }
                SmtpState::Data => {
                    if line.trim() == "." {
                        Connection::send_commands(&mut framed, vec!["250 Ok".to_string()]).await?;
                        state = SmtpState::Quit;
                    } else if parsing_headers && line.is_empty() {
                        parsing_headers = false;
                    } else if parsing_headers {
                        match crate::parser::parse_header(line.as_str()) {
                            Ok((_, h)) => {
                                headers.push(h);
                            }
                            Err(e) => {
                                log::error!("Failed to parse header: {:?}", e);
                                Connection::send_commands(
                                    &mut framed,
                                    vec!["500 Unrecognized header".to_string()],
                                )
                                .await?;
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
                            Connection::send_commands(
                                &mut framed,
                                vec!["500 Unrecognized command".to_string()],
                            )
                            .await?;
                            break;
                        }
                    };

                    match command {
                        SmtpCommand::Quit => {
                            Connection::send_commands(&mut framed, vec!["221 Bye".to_string()])
                                .await?;
                        }
                        _ => {
                            Connection::send_commands(
                                &mut framed,
                                vec!["500 Unrecognized command".to_string()],
                            )
                            .await?;
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
}
