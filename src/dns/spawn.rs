use core::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use hickory_server::ServerFuture;

use crate::config::Config;
use crate::dns::error::DnsError;
use crate::dns::server::Handler;
use crate::dns::tls::{DynamicCertResolver, tls_server_config};
use tokio::net::TcpListener;
use tokio::runtime::Runtime;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

pub fn start_dns_server(config: Config) -> std::thread::JoinHandle<Result<(), DnsError>> {
    std::thread::spawn(move || {
        let rt = match Runtime::new() {
            Ok(runtime) => runtime,
            Err(e) => return Err(DnsError::TokioRuntimeCreation(e)),
        };

        rt.block_on(async move {
            let cancel_token = CancellationToken::new();
            let mut tasks: Vec<JoinHandle<Result<(), DnsError>>> = Vec::new();
            let handler = Handler::new(config.clone());

            let ctrl_c_cancel_token = cancel_token.clone();
            tokio::spawn(async move {
                tokio::signal::ctrl_c()
                    .await
                    .unwrap_or_else(|e| tracing::error!("Failed to listen for Ctrl+C: {}", e));
                tracing::info!("Ctrl+C received, shutting down...");
                ctrl_c_cancel_token.cancel();
            });

            // UDP Server
            let udp_addr: SocketAddr = match config.listen.udp.parse() {
                Ok(addr) => addr,
                Err(e) => {
                    return Err(DnsError::ParseListenAddress(format!(
                        "UDP listen address: {}",
                        e
                    )));
                }
            };
            let udp_handler = handler.clone();
            let udp_cancel_token = cancel_token.clone();
            tasks.push(tokio::spawn(async move {
                let mut server = ServerFuture::new(udp_handler);
                if let Err(e) = tokio::net::UdpSocket::bind(udp_addr)
                    .await
                    .map(|udp_socket| server.register_socket(udp_socket))
                {
                    return Err(DnsError::UdpSocketBind(udp_addr.to_string(), e));
                }
                tracing::info!("Listening for UDP DNS on {}", udp_addr);
                tokio::select! {
                    _ = udp_cancel_token.cancelled() => {
                        tracing::info!("UDP server shutting down.");
                        Ok(())
                    }
                    result = server.block_until_done() => {
                        result.map_err(|e| DnsError::DnsServer(e.to_string()))
                    }
                }
            }));

            // TCP Server
            let tcp_addr: SocketAddr = match config.listen.tcp.parse() {
                Ok(addr) => addr,
                Err(e) => {
                    return Err(DnsError::ParseListenAddress(format!(
                        "TCP listen address: {}",
                        e
                    )));
                }
            };
            let tcp_handler = handler.clone();
            let tcp_cancel_token = cancel_token.clone();
            tasks.push(tokio::spawn(async move {
                let mut server = ServerFuture::new(tcp_handler);
                if let Err(e) = tokio::net::TcpListener::bind(tcp_addr)
                    .await
                    .map(|tcp_listener| {
                        server.register_listener(tcp_listener, Duration::from_secs(10))
                    })
                {
                    return Err(DnsError::TcpSocketBind(tcp_addr.to_string(), e));
                }
                tracing::info!("Listening for TCP DNS on {}", tcp_addr);
                tokio::select! {
                    _ = tcp_cancel_token.cancelled() => {
                        tracing::info!("TCP server shutting down.");
                        Ok(())
                    }
                    result = server.block_until_done() => {
                        result.map_err(|e| DnsError::DnsServer(e.to_string()))
                    }
                }
            }));

            // HTTP Server (DoH)
            let http_addr: SocketAddr = match config.listen.http.parse() {
                Ok(addr) => addr,
                Err(e) => {
                    return Err(DnsError::ParseListenAddress(format!(
                        "HTTP listen address: {}",
                        e
                    )));
                }
            };
            let http_cancel_token = cancel_token.clone();
            let http_handler = handler.clone();
            let http_config = config.clone();
            tasks.push(tokio::spawn(async move {
                tracing::info!("Listening for HTTP on {}", http_addr);

                if let Some(tls_cert_config) = http_config.tls_cert_config {
                    let resolver_config = Arc::new(DynamicCertResolver::new(tls_cert_config));
                    let mut server = ServerFuture::new(http_handler);
                    if let Err(e) = TcpListener::bind(http_addr).await.map(|tcp_listener| {
                        server.register_https_listener(
                            tcp_listener,
                            Duration::from_secs(30),
                            resolver_config,
                            None, // endpoint_name, not used in director
                            http_config
                                .https_endpoint
                                .unwrap_or_else(|| "/dns-query".to_string()),
                        )
                    }) {
                        return Err(DnsError::TcpSocketBind(http_addr.to_string(), e));
                    }
                    tracing::info!("Listening for HTTPS DNS on {}", http_addr);

                    tokio::select! {
                        _ = http_cancel_token.cancelled() => {
                            tracing::info!("HTTPS server shutting down.");
                            Ok(())
                        }
                        result = server.block_until_done() => {
                            result.map_err(|e| DnsError::DnsServer(e.to_string()))
                        }
                    }
                } else {
                    tracing::info!("HTTPS server disabled: no TLS certificate configured.");
                    http_cancel_token.cancelled().await;
                    Ok(())
                }
            }));

            // TLS Server (DoT)
            let tls_addr: SocketAddr = match config.listen.tls.parse() {
                Ok(addr) => addr,
                Err(e) => {
                    return Err(DnsError::ParseListenAddress(format!(
                        "TLS listen address: {}",
                        e
                    )));
                }
            };
            let tls_cancel_token = cancel_token.clone();
            let tls_handler = handler.clone();
            let tls_config = config.clone();
            tasks.push(tokio::spawn(async move {
                tracing::info!("Listening for TLS on {}", tls_addr);

                if let Some(tls_cert_config) = tls_config.tls_cert_config {
                    let resolver_config = Arc::new(DynamicCertResolver::new(tls_cert_config));
                    let serv_conf = tls_server_config(b"dot", resolver_config)
                        .map_err(|e| DnsError::TlsConfig(e.to_string()))?;
                    let mut server = ServerFuture::new(tls_handler);
                    if let Err(e) = TcpListener::bind(tls_addr).await.map(|tcp_listener| {
                        server.register_tls_listener_with_tls_config(
                            tcp_listener,
                            Duration::from_secs(30),
                            Arc::new(serv_conf),
                        )
                    }) {
                        return Err(DnsError::TcpSocketBind(tls_addr.to_string(), e));
                    }
                    tracing::info!("Listening for TLS DNS on {}", tls_addr);

                    tokio::select! {
                        _ = tls_cancel_token.cancelled() => {
                            tracing::info!("TLS server shutting down.");
                            Ok(())
                        }
                        result = server.block_until_done() => {
                            result.map_err(|e| DnsError::DnsServer(e.to_string()))
                        }
                    }
                } else {
                    tracing::info!("TLS server disabled: no TLS certificate configured.");
                    tls_cancel_token.cancelled().await;
                    Ok(())
                }
            }));

            for task in tasks {
                task.await
                    .map_err(|e| DnsError::DnsServer(format!("Server task panicked: {}", e)))??;
            }

            Ok(())
        })
    })
}
