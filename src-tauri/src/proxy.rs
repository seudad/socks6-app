use crate::profiles::Profile;
use anyhow::{bail, Context, Result};
use serde::Serialize;
use std::io::ErrorKind;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::Emitter;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream, UdpSocket};
use tokio::sync::{Mutex, Notify, Semaphore};
use tokio_rustls::rustls;
use tokio_rustls::TlsConnector;

#[derive(Debug, Clone, Serialize)]
pub struct ProxyStatus {
    pub connected: bool,
    pub profile_id: Option<String>,
    pub listen_addr: Option<String>,
    pub server: Option<String>,
    pub uptime_secs: u64,
    pub connections: u64,
}

#[derive(Debug, Clone, Serialize)]
struct LogEvent {
    level: String,
    message: String,
}

struct RunningState {
    profile_id: String,
    listen_addr: String,
    server: String,
    started_at: Instant,
    connections: AtomicU64,
    stop: Arc<Notify>,
}

pub struct ProxyEngine {
    running: Arc<Mutex<Option<Arc<RunningState>>>>,
}

impl ProxyEngine {
    pub fn new() -> Self {
        Self {
            running: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn status(&self) -> ProxyStatus {
        let guard = self.running.lock().await;
        match guard.as_ref() {
            Some(state) => ProxyStatus {
                connected: true,
                profile_id: Some(state.profile_id.clone()),
                listen_addr: Some(state.listen_addr.clone()),
                server: Some(state.server.clone()),
                uptime_secs: state.started_at.elapsed().as_secs(),
                connections: state.connections.load(Ordering::Relaxed),
            },
            None => ProxyStatus {
                connected: false,
                profile_id: None,
                listen_addr: None,
                server: None,
                uptime_secs: 0,
                connections: 0,
            },
        }
    }

    pub async fn start(&self, profile: Profile, app: tauri::AppHandle) -> Result<()> {
        let mut guard = self.running.lock().await;
        if guard.is_some() {
            bail!("proxy already running — disconnect first");
        }

        let listen_addr: SocketAddr = profile
            .listen
            .parse()
            .context("invalid listen address")?;

        let secret = decode_secret(&profile.secret)?;
        let short_id = decode_short_id(&profile.short_id)?;

        let auth = if !profile.auth_user.is_empty() && !profile.auth_pass.is_empty() {
            Some((profile.auth_user.clone(), profile.auth_pass.clone()))
        } else {
            None
        };

        let listener = TcpListener::bind(listen_addr)
            .await
            .context("failed to bind listen address")?;

        let actual_addr = listener.local_addr()?;

        let state = Arc::new(RunningState {
            profile_id: profile.id.clone(),
            listen_addr: actual_addr.to_string(),
            server: profile.server.clone(),
            started_at: Instant::now(),
            connections: AtomicU64::new(0),
            stop: Arc::new(Notify::new()),
        });

        *guard = Some(state.clone());
        drop(guard);

        let tls_config = build_tls_config()?;
        let tls_config = Arc::new(tls_config);
        let max_tls = profile.max_tls_parallel.max(1);
        let tls_slots = Arc::new(Semaphore::new(max_tls));

        let engine_running = self.running.clone();
        let stop = state.stop.clone();
        let conn_counter = state.clone();
        let app_clone = app.clone();

        emit_log(&app, "info", &format!("Proxy started on {actual_addr} → {}", profile.server));

        let proxy_config = Arc::new(ProxyConfig {
            server: profile.server.clone(),
            server_name: profile.server_name.clone(),
            secret,
            short_id,
            auth,
            auth_time_offset_secs: profile.auth_time_offset_secs,
        });

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    result = listener.accept() => {
                        match result {
                            Ok((stream, peer)) => {
                                stream.set_nodelay(true).ok();
                                conn_counter.connections.fetch_add(1, Ordering::Relaxed);
                                let cfg = proxy_config.clone();
                                let tls_cfg = tls_config.clone();
                                let slots = tls_slots.clone();
                                let app = app_clone.clone();

                                tokio::spawn(async move {
                                    if let Err(e) = handle_local_client(stream, peer, cfg, tls_cfg, slots).await {
                                        emit_log(&app, "warn", &format!("{peer}: {e:#}"));
                                    }
                                });
                            }
                            Err(e) => {
                                emit_log(&app_clone, "error", &format!("accept error: {e}"));
                            }
                        }
                    }
                    _ = stop.notified() => {
                        emit_log(&app_clone, "info", "Proxy stopped");
                        break;
                    }
                }
            }

            let mut guard = engine_running.lock().await;
            *guard = None;
        });

        Ok(())
    }

    pub async fn stop(&self) -> Result<()> {
        let mut guard = self.running.lock().await;
        match guard.take() {
            Some(state) => {
                state.stop.notify_one();
                Ok(())
            }
            None => bail!("proxy is not running"),
        }
    }
}

struct ProxyConfig {
    server: String,
    server_name: String,
    secret: [u8; 32],
    short_id: [u8; 8],
    auth: Option<(String, String)>,
    auth_time_offset_secs: i64,
}

fn emit_log(app: &tauri::AppHandle, level: &str, message: &str) {
    app.emit(
        "proxy-log",
        LogEvent {
            level: level.to_string(),
            message: message.to_string(),
        },
    )
    .ok();
}

fn decode_secret(b64: &str) -> Result<[u8; 32]> {
    let bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, b64)
        .context("invalid base64 secret")?;
    if bytes.len() != 32 {
        bail!("secret must be 32 bytes, got {}", bytes.len());
    }
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&bytes);
    Ok(arr)
}

fn decode_short_id(hex: &str) -> Result<[u8; 8]> {
    let bytes = socks6::reality::hex_decode(hex).context("invalid hex short_id")?;
    if bytes.len() != 8 {
        bail!("short_id must be 8 bytes (16 hex chars), got {}", bytes.len());
    }
    let mut arr = [0u8; 8];
    arr.copy_from_slice(&bytes);
    Ok(arr)
}

fn build_tls_config() -> Result<rustls::ClientConfig> {
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let config = rustls::ClientConfig::builder_with_provider(provider)
        .with_protocol_versions(&[&rustls::version::TLS13])
        .context("TLS 1.3 config")?
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(NoCertVerify))
        .with_no_client_auth();
    Ok(config)
}

#[derive(Debug)]
struct NoCertVerify;

impl rustls::client::danger::ServerCertVerifier for NoCertVerify {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[rustls::pki_types::CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        rustls::crypto::ring::default_provider()
            .signature_verification_algorithms
            .supported_schemes()
    }
}

// ── TLS tunnel establishment ────────────────────────────────────────────

async fn establish_server_tunnel(
    config: &ProxyConfig,
    tls_config: Arc<rustls::ClientConfig>,
    tls_slots: &Semaphore,
) -> Result<tokio_rustls::client::TlsStream<TcpStream>> {
    let _tls_slot = tls_slots.acquire().await.context("TLS semaphore closed")?;

    let connector = TlsConnector::from(tls_config);
    const TCP_CONNECT_TIMEOUT: Duration = Duration::from_secs(20);
    const TLS_HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(25);
    const MAX_TRIES: u32 = 3;

    let mut attempt: u32 = 0;
    let mut tunnel = loop {
        attempt += 1;
        let tcp = match tokio::time::timeout(TCP_CONNECT_TIMEOUT, TcpStream::connect(&config.server)).await {
            Ok(Ok(t)) => t,
            Ok(Err(e)) => return Err(e).context(format!("TCP to {}", config.server)),
            Err(_) => bail!("TCP to {}: timeout {}s", config.server, TCP_CONNECT_TIMEOUT.as_secs()),
        };
        tcp.set_nodelay(true).ok();

        let name = rustls::pki_types::ServerName::try_from(config.server_name.clone())
            .context("invalid SNI")?;

        match tokio::time::timeout(TLS_HANDSHAKE_TIMEOUT, connector.connect(name, tcp)).await {
            Ok(Ok(t)) => break t,
            Ok(Err(e)) => {
                let retriable = matches!(
                    e.kind(),
                    ErrorKind::UnexpectedEof | ErrorKind::ConnectionReset | ErrorKind::ConnectionAborted
                ) || e.to_string().to_lowercase().contains("eof");
                if retriable && attempt < MAX_TRIES {
                    tokio::time::sleep(Duration::from_millis(40 + 60 * attempt as u64)).await;
                    continue;
                }
                return Err(e).context("TLS handshake");
            }
            Err(_) => {
                if attempt < MAX_TRIES {
                    tokio::time::sleep(Duration::from_millis(40 + 60 * attempt as u64)).await;
                    continue;
                }
                bail!("TLS to {}: timeout {}s", config.server, TLS_HANDSHAKE_TIMEOUT.as_secs());
            }
        }
    };

    socks6::reality::send_tunnel_auth(
        &mut tunnel,
        &config.secret,
        &config.short_id,
        config.auth_time_offset_secs,
    )
    .await
    .context("Reality authentication")?;

    Ok(tunnel)
}

// ── Per-connection handler ──────────────────────────────────────────────

async fn handle_local_client(
    mut local: TcpStream,
    peer: SocketAddr,
    config: Arc<ProxyConfig>,
    tls_config: Arc<rustls::ClientConfig>,
    tls_slots: Arc<Semaphore>,
) -> Result<()> {
    local_socks5_handshake(&mut local).await?;
    let cmd = local_socks5_read_request(&mut local).await?;

    match cmd {
        LocalSocksCommand::Connect { host, port } => {
            tracing::info!(%peer, target = %format!("{host}:{port}"), "CONNECT");
            let mut tunnel = establish_server_tunnel(&config, tls_config, &tls_slots).await?;
            remote_socks6_connect(&mut tunnel, &host, port, config.auth.as_ref()).await?;
            local_socks5_send_ok(&mut local).await?;
            tokio::io::copy_bidirectional(&mut local, &mut tunnel).await?;
        }
        LocalSocksCommand::UdpAssociate => {
            tracing::info!(%peer, "UDP ASSOCIATE");
            let bind_ip = if local.local_addr()?.ip().is_loopback() { "127.0.0.1" } else { "0.0.0.0" };
            let local_udp = UdpSocket::bind(format!("{bind_ip}:0")).await?;
            let udp_addr = local_udp.local_addr()?;
            let mut tunnel = establish_server_tunnel(&config, tls_config, &tls_slots).await?;
            remote_socks6_udp_associate(&mut tunnel, config.auth.as_ref()).await?;
            local_socks5_send_ok_addr(&mut local, udp_addr).await?;
            socks6::udp_relay::run_client_tunneled(tunnel, local_udp, local).await?;
        }
    }

    Ok(())
}

// ── Local SOCKS5 protocol (app-facing) ──────────────────────────────────

enum LocalSocksCommand {
    Connect { host: String, port: u16 },
    UdpAssociate,
}

async fn local_socks5_handshake(stream: &mut TcpStream) -> Result<()> {
    let mut hdr = [0u8; 2];
    stream.read_exact(&mut hdr).await.context("SOCKS5 greeting")?;
    if hdr[0] != 0x05 {
        bail!("not SOCKS5: {:#x}", hdr[0]);
    }
    let n = hdr[1] as usize;
    let mut methods = vec![0u8; n];
    stream.read_exact(&mut methods).await?;
    if methods.contains(&0x00) {
        stream.write_all(&[0x05, 0x00]).await?;
        stream.flush().await?;
    } else {
        stream.write_all(&[0x05, 0xFF]).await?;
        bail!("client does not support no-auth method");
    }
    Ok(())
}

async fn local_socks5_read_request(stream: &mut TcpStream) -> Result<LocalSocksCommand> {
    let mut hdr = [0u8; 4];
    stream.read_exact(&mut hdr).await?;
    if hdr[0] != 0x05 {
        bail!("not SOCKS5: {:#x}", hdr[0]);
    }
    let cmd = hdr[1];
    if cmd != 0x01 && cmd != 0x03 {
        stream.write_all(&[0x05, 0x07, 0x00, 0x01, 0, 0, 0, 0, 0, 0]).await.ok();
        bail!("unsupported command: {:#x}", cmd);
    }

    let (host, port) = match hdr[3] {
        0x01 => {
            let mut ip = [0u8; 4];
            stream.read_exact(&mut ip).await?;
            let port = stream.read_u16().await?;
            (format!("{}.{}.{}.{}", ip[0], ip[1], ip[2], ip[3]), port)
        }
        0x03 => {
            let len = stream.read_u8().await? as usize;
            let mut domain = vec![0u8; len];
            stream.read_exact(&mut domain).await?;
            let port = stream.read_u16().await?;
            (String::from_utf8(domain).context("invalid domain")?, port)
        }
        0x04 => {
            let mut ip = [0u8; 16];
            stream.read_exact(&mut ip).await?;
            let port = stream.read_u16().await?;
            let addr = std::net::Ipv6Addr::from(ip);
            (format!("[{addr}]"), port)
        }
        other => bail!("unsupported ATYP: {other:#x}"),
    };

    match cmd {
        0x01 => Ok(LocalSocksCommand::Connect { host, port }),
        0x03 => Ok(LocalSocksCommand::UdpAssociate),
        _ => unreachable!(),
    }
}

async fn local_socks5_send_ok(stream: &mut TcpStream) -> Result<()> {
    stream.write_all(&[0x05, 0x00, 0x00, 0x01, 0, 0, 0, 0, 0, 0]).await?;
    stream.flush().await?;
    Ok(())
}

async fn local_socks5_send_ok_addr(stream: &mut TcpStream, addr: SocketAddr) -> Result<()> {
    let mut buf = Vec::with_capacity(22);
    buf.extend_from_slice(&[0x05, 0x00, 0x00]);
    match addr {
        SocketAddr::V4(a) => {
            buf.push(0x01);
            buf.extend_from_slice(&a.ip().octets());
            buf.extend_from_slice(&a.port().to_be_bytes());
        }
        SocketAddr::V6(a) => {
            buf.push(0x04);
            buf.extend_from_slice(&a.ip().octets());
            buf.extend_from_slice(&a.port().to_be_bytes());
        }
    }
    stream.write_all(&buf).await?;
    stream.flush().await?;
    Ok(())
}

// ── Remote SOCKS6 protocol (server-facing) ──────────────────────────────

async fn remote_socks6_connect<S>(
    stream: &mut S,
    host: &str,
    port: u16,
    auth: Option<&(String, String)>,
) -> Result<()>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    if auth.is_some() {
        stream.write_all(&[0x06, 0x01, 0x02]).await?;
    } else {
        stream.write_all(&[0x06, 0x01, 0x00]).await?;
    }

    let mut choice = [0u8; 2];
    stream.read_exact(&mut choice).await?;
    if choice[0] != 0x06 {
        bail!("server not SOCKS6: {:#x}", choice[0]);
    }

    match choice[1] {
        0x00 => {}
        0x02 => {
            let (user, pass) = auth.context("server requires auth")?;
            let mut msg = Vec::with_capacity(3 + user.len() + pass.len());
            msg.push(0x01);
            msg.push(user.len() as u8);
            msg.extend_from_slice(user.as_bytes());
            msg.push(pass.len() as u8);
            msg.extend_from_slice(pass.as_bytes());
            stream.write_all(&msg).await?;
            let mut resp = [0u8; 2];
            stream.read_exact(&mut resp).await?;
            if resp[1] != 0x00 {
                bail!("server auth failed");
            }
        }
        0xFF => bail!("server rejected auth methods"),
        other => bail!("unsupported method: {other:#x}"),
    }

    let mut req = Vec::with_capacity(4 + 1 + host.len() + 4);
    req.extend_from_slice(&[0x06, 0x01]);
    if let Ok(v4) = host.parse::<std::net::Ipv4Addr>() {
        req.push(0x01);
        req.extend_from_slice(&v4.octets());
    } else if let Ok(v6) = host.trim_matches(|c| c == '[' || c == ']').parse::<std::net::Ipv6Addr>() {
        req.push(0x04);
        req.extend_from_slice(&v6.octets());
    } else {
        req.push(0x03);
        req.push(host.len() as u8);
        req.extend_from_slice(host.as_bytes());
    }
    req.extend_from_slice(&port.to_be_bytes());
    req.extend_from_slice(&0u16.to_be_bytes());
    stream.write_all(&req).await?;

    let mut reply = [0u8; 3];
    stream.read_exact(&mut reply).await?;
    if reply[0] != 0x06 { bail!("invalid SOCKS6 reply: {:#x}", reply[0]); }
    if reply[1] != 0x00 { bail!("CONNECT rejected: {:#x}", reply[1]); }

    match reply[2] {
        0x01 => { let mut skip = [0u8; 6]; stream.read_exact(&mut skip).await?; }
        0x03 => {
            let len = stream.read_u8().await? as usize;
            let mut skip = vec![0u8; len + 2];
            stream.read_exact(&mut skip).await?;
        }
        0x04 => { let mut skip = [0u8; 18]; stream.read_exact(&mut skip).await?; }
        atyp => bail!("unknown ATYP in reply: {atyp:#x}"),
    }

    let opts_len = stream.read_u16().await? as usize;
    if opts_len > 0 {
        let mut skip = vec![0u8; opts_len];
        stream.read_exact(&mut skip).await?;
    }

    Ok(())
}

async fn remote_socks6_udp_associate<S>(
    stream: &mut S,
    auth: Option<&(String, String)>,
) -> Result<()>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    if auth.is_some() {
        stream.write_all(&[0x06, 0x01, 0x02]).await?;
    } else {
        stream.write_all(&[0x06, 0x01, 0x00]).await?;
    }

    let mut choice = [0u8; 2];
    stream.read_exact(&mut choice).await?;
    if choice[0] != 0x06 { bail!("server not SOCKS6: {:#x}", choice[0]); }

    match choice[1] {
        0x00 => {}
        0x02 => {
            let (user, pass) = auth.context("server requires auth")?;
            let mut msg = Vec::with_capacity(3 + user.len() + pass.len());
            msg.push(0x01);
            msg.push(user.len() as u8);
            msg.extend_from_slice(user.as_bytes());
            msg.push(pass.len() as u8);
            msg.extend_from_slice(pass.as_bytes());
            stream.write_all(&msg).await?;
            let mut resp = [0u8; 2];
            stream.read_exact(&mut resp).await?;
            if resp[1] != 0x00 { bail!("server auth failed"); }
        }
        0xFF => bail!("server rejected auth methods"),
        other => bail!("unsupported method: {other:#x}"),
    }

    stream.write_all(&[0x06, 0x03, 0x01, 0, 0, 0, 0, 0, 0, 0, 0]).await?;

    let mut reply = [0u8; 3];
    stream.read_exact(&mut reply).await?;
    if reply[0] != 0x06 { bail!("invalid SOCKS6 reply: {:#x}", reply[0]); }
    if reply[1] != 0x00 { bail!("UDP ASSOCIATE rejected: {:#x}", reply[1]); }

    match reply[2] {
        0x01 => { let mut skip = [0u8; 6]; stream.read_exact(&mut skip).await?; }
        0x03 => {
            let len = stream.read_u8().await? as usize;
            let mut skip = vec![0u8; len + 2];
            stream.read_exact(&mut skip).await?;
        }
        0x04 => { let mut skip = [0u8; 18]; stream.read_exact(&mut skip).await?; }
        atyp => bail!("unknown ATYP in reply: {atyp:#x}"),
    }

    let opts_len = stream.read_u16().await? as usize;
    if opts_len > 0 {
        let mut skip = vec![0u8; opts_len];
        stream.read_exact(&mut skip).await?;
    }

    Ok(())
}
