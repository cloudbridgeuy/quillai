mod prelude;

use futures::{stream::iter, SinkExt, StreamExt};
use native_tls::{Identity, TlsAcceptor};
use openssl::{
    asn1::Asn1Time,
    pkey::PKey,
    rsa::Rsa,
    x509::{
        extension::{AuthorityKeyIdentifier, BasicConstraints, SubjectKeyIdentifier},
        X509NameBuilder, X509,
    },
};
use std::net::SocketAddr;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::net::{TcpListener, TcpStream};
use tokio_native_tls::{TlsAcceptor as TokioTlsAcceptor, TlsStream};
use tokio_util::codec::{Framed, LinesCodec, LinesCodecError};

use crate::prelude::*;

async fn listen() -> Result<()> {
    let addr = SocketAddr::from(([127, 0, 0, 1], 2525));
    let listener = TcpListener::bind(addr).await?;

    log::trace!("SMTP server listening on {}", addr);

    loop {
        match listener.accept().await {
            Ok((mut stream, _)) => {
                tokio::spawn(async move {
                    if let Err(err) = handle_connection(&mut stream).await {
                        log::trace!("Error handling STMP connection: {:?}", err);
                    };
                });
            }
            Err(err) => {
                anyhow::bail!("Error establishing SMTP connection: {:?}", err);
            }
        }
    }
}

async fn handle_connection(stream: &mut TcpStream) -> Result<()> {
    let (reader, writer) = stream.split();
    let mut _reader = BufReader::new(reader);
    let mut writer = BufWriter::new(writer);

    writer
        .write_all(format!("220 {}\r\n", "My SMTP Server").as_bytes())
        .await?;

    Ok(())
}

#[derive(Debug, Clone, PartialEq)]
enum SmtpState {
    // State after the connection is established and the client is waiting for a command.
    Command,
    // State after the DATA command has been received and the email content is being accumulated.
    Data,
    // State after the QUIT command has been received, the connection is closed.
    Quit,
}

async fn handle_session(stream: TcpStream) -> anyhow::Result<()> {
    let re_smtp_mail = regex::Regex::new(r"(?i)from: ?<(.+)>").unwrap();
    let re_smtp_rcpt = regex::Regex::new(r"(?i)to: ?<(.+)>").unwrap();

    let mut message = String::new();
    let mut state = SmtpState::Command;
    let mut mailfrom: Option<String> = None;
    let mut rcpts: Vec<String> = Vec::new();
    let mut framed = Framed::new(stream, LinesCodec::new());

    while let Some(line_str) = framed.next().await {
        let line = line_str?;

        match state {
            SmtpState::Command => {
                let space_pos = line.find(" ").unwrap_or(line.len());
                let (command, arg) = line.split_at(space_pos);
                let arg = arg.trim();
                match &*command.trim().to_uppercase() {
                    "HELO" | "EHLO" => {
                        send_commands(&mut framed, vec!["250 Hello".to_string()]).await?;
                    }
                    "MAIL" => {
                        // Handle MAIL FROM command
                        if let Some(address) = re_smtp_mail.captures(arg).and_then(|cap| cap.get(1))
                        {
                            mailfrom = Some(address.as_str().to_string());
                            send_commands(&mut framed, vec!["250 OK".to_string()]).await?;
                        } else {
                            send_commands(
                                &mut framed,
                                vec!["501 Syntax: MAIL From: <address>".to_string()],
                            )
                            .await?;
                        }
                    }
                    "RCPT" => {
                        // Handle RCPT TO command
                        if mailfrom.is_none() {
                            send_commands(
                                &mut framed,
                                vec!["503 Error: Send MAIL first".to_string()],
                            )
                            .await?;
                        } else if let Some(address) =
                            re_smtp_rcpt.captures(arg).and_then(|cap| cap.get(1))
                        {
                            rcpts.push(address.as_str().to_string());
                            send_commands(&mut framed, vec!["250 OK".to_string()]).await?;
                        } else {
                            send_commands(
                                &mut framed,
                                vec!["501 Syntax: RCPT TO: <address>".to_string()],
                            )
                            .await?;
                        }
                    }
                    "DATA" => {
                        if rcpts.is_empty() {
                            send_commands(&mut framed, vec!["503 Error: MAIL FROM and RCPT TO must be set before sending DATA".to_string()]).await?;
                        } else {
                            state = SmtpState::Data;
                            send_commands(
                                &mut framed,
                                vec!["354 End data with <CR><LF>.<CR><LF>".to_string()],
                            )
                            .await?;
                        }
                    }
                    "NOOP" => {
                        send_commands(&mut framed, vec!["250 OK".to_string()]).await?;
                    }
                    "RSET" => {
                        mailfrom = None;
                        rcpts = Vec::new();
                        message = String::new();
                        send_commands(&mut framed, vec!["250 OK".to_string()]).await?;
                    }
                    "QUIT" => {
                        send_commands(&mut framed, vec!["221 Bye".to_string()]).await?;
                        state = SmtpState::Quit;
                    }
                    _ => {
                        send_commands(&mut framed, vec!["500 Unknown command".to_string()]).await?;
                    }
                }
            }
            SmtpState::Data => {
                if line.trim() == "." {
                    // The end of the email content has been received
                    send_commands(&mut framed, vec!["250 OK".to_string()]).await?;
                    // reset the state and variables for the next email
                    mailfrom = None;
                    rcpts = Vec::new();
                    message = String::new();
                    state = SmtpState::Command;
                    // we can now handle the email:
                    handle_email(mailfrom.clone(), &rcpts, &message);
                } else {
                    // Add the received line to the email content
                    message.push_str(&line);
                    message.push('\n');
                }
            }
            SmtpState::Quit => {
                break;
            }
        };
    }

    Ok(())
}

async fn handle_unsecured_session(
    reader: &mut BufReader<TcpStream>,
    writer: &mut BufWriter<TcpStream>,
) -> anyhow::Result<()> {
    let mut is_tls = false;
    let mut line = String::new();

    while reader.read_line(&mut line).await? != 0 {
        let space_pos = line.find(" ").unwrap_or(line.len());
        let (command, _) = line.split_at(space_pos);

        match command.trim().to_uppercase().as_ref() {
            "EHLO" | "HELO" => {
                writer.write_all(b"250-windmill Hello\r\n").await?;
                writer.write_all(b"250-STARTTLS\r\n").await?;
                writer.write_all(b"250 What you've got?\r\n").await?;
                writer.flush().await?;
            }
            "STARTTLS" => {
                writer.write_all(b"220 GO ON\r\n").await?;
                writer.flush().await?;
                is_tls = true;
                break;
            }
            "QUIT" => {
                writer.write_all(b"221 Have a nice day!\r\n").await?;
                writer.flush().await?;
                break;
            }
            "NOOP" => {
                writer.write_all(b"250 OK\r\n").await?;
                writer.flush().await?;
            }
            "MAIL" | "RCPT" | "DATA" | "RSET" => {
                writer
                    .write_all(b"530 Must issue a STARTTLS command first\r\n")
                    .await?;
                writer.flush().await?;
            }
            _ => {
                writer.write_all(b"500 Unknown command\r\n").await?;
                writer.flush().await?;
            }
        }

        line.clear();
    }

    if is_tls {
        handle_starttls(stream).await?;
    }
}
async fn send_commands(
    framed: &mut Framed<TcpStream, LinesCodec>,
    commands: Vec<String>,
) -> Result<()> {
    for command in commands {
        framed.send(command).await?;
    }
    Ok(())
}

fn handle_email(mailfrom: Option<String>, rcpts: &[String], message: &str) {
    if let Some(mailfrom) = mailfrom {
        log::info!("New email from: {}", mailfrom);
        log::info!("Recipients: {:?}", rcpts);
        log::info!("Message content: {}", message);

        // Example: Store or further process the email here.
        // This could involve writing to a database, file, or invoking another service/API.
    } else {
        log::warn!("Email handling attempted without 'MAIL FROM' specification.");
    }
}

async fn handle_starttls(stream: &mut TcpStream) -> Result<()> {
    // ideally the certificate should only be loaded from here and not generated each time
    let (pem_certificate, pem_private_key) = generate_certificate()?;
    let identity = Identity::from_pkcs8(&pem_certificate, &pem_private_key)?;
    let tls_acceptor = TlsAcceptor::builder(identity).build()?;
    let tls_acceptor = TokioTlsAcceptor::from(tls_acceptor);

    match tls_acceptor.accept(stream).await {
        Ok(stream) => {
            // we can now handle the normal SMTP session
            handle_session(stream).await?;
        }
        Err(e) => {
            tracing::error!("Error establishing SMTP TLS connection: {:?}", e);
        }
    };

    Ok(())
}

fn  generate_certificate<'a>() -> Result<(&'a [u8], &'a[u8])> {
    let cert_result = {
        let rsa = Rsa::generate(4096)?;
        let pkey = PKey::from_rsa(rsa)?;
        let mut name = X509NameBuilder::new()?;
        name.append_entry_by_text("CN", "localhost")?;
        let name = name.build();
        let mut builder = X509::builder()?;
        builder.set_version(2)?;
        builder.set_subject_name(&name)?;
        builder.set_issuer_name(&name)?;
        builder.set_pubkey(&pkey)?;
        let now = Asn1Time::days_from_now(0)?;
        let later = Asn1Time::days_from_now(3650)?;
        builder.set_not_before(now.as_ref())?;
        builder.set_not_after(later.as_ref())?;
        builder.append_extension(BasicConstraints::new().critical().ca().build()?)?;
        builder.append_extension(SubjectKeyIdentifier::new().build(&builder.x509v3_context(None, None))?)?;
        builder.append_extension(AuthorityKeyIdentifier::new().keyid(true).issuer(true).build(&builder.x509v3_context(None, None))?)?;
        builder.sign(&pkey, openssl::hash::MessageDigest::sha256())?;
        let c = builder.build();
        Ok((c.to_pem()?, pkey.private_key_to_pem_pkcs8()?))
    }
    let (pem_certificate, pem_private_key) = cert_result
        .as_ref()
        .map_err(|e| anyhow::anyhow!("Could not generate self-signed certificates: {}", e))?;

    Ok((pem_certificate, pem_private_key))
}

#[tokio::main]
async fn main() -> Result<()> {
    Ok(())
}
