use core::net::SocketAddr;
use std::time::Duration;

use hickory_server::ServerFuture;
use tokio::runtime::Runtime;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::config::Config;
use crate::dns::error::DnsError;
use crate::dns::server::Handler;

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
                    .expect("Failed to listen for Ctrl+C");
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

            // HTTP Server (Placeholder)
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
            tasks.push(tokio::spawn(async move {
                tracing::info!("Listening for HTTP on {}", http_addr);
                // TODO: Implement actual HTTP server
                http_cancel_token.cancelled().await;
                tracing::info!("HTTP server shutting down.");
                Ok(())
            }));

            // TLS Server (Placeholder)
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
            tasks.push(tokio::spawn(async move {
                tracing::info!("Listening for TLS on {}", tls_addr);
                // TODO: Implement actual TLS server
                tls_cancel_token.cancelled().await;
                tracing::info!("TLS server shutting down.");
                Ok(())
            }));

            for task in tasks {
                task.await
                    .map_err(|e| DnsError::DnsServer(format!("Server task panicked: {}", e)))??;
            }

            Ok(())
        })
    })
}
