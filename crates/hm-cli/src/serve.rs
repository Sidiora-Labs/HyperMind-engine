use anyhow::{Context, Result, anyhow};
use clap::Args;
use hm_serve::config::ServerConfig;
use hm_serve::grpc::{Gateway, GrpcServer, ListenerRole, TlsIdentity};
use hm_serve::rest::RestServer;
use hm_serve::uds::UdsServer;
use serde_json::{Value, json};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::watch;
use tokio::task::JoinSet;

#[derive(Debug, Default, Args)]
pub(crate) struct RemoteOptions {
    #[arg(long, requires = "tls_source")]
    grpc_bind: Option<SocketAddr>,
    #[arg(long, requires = "tls_source")]
    grpc_admin_bind: Option<SocketAddr>,
    #[arg(long, requires = "tls_source")]
    rest_bind: Option<SocketAddr>,
    #[arg(long, requires = "tls_source")]
    rest_admin_bind: Option<SocketAddr>,
    #[arg(long, group = "tls_source", requires_all = ["tls_key", "tls_client_ca"])]
    tls_cert: Option<PathBuf>,
    #[arg(long, requires_all = ["tls_cert", "tls_client_ca"])]
    tls_key: Option<PathBuf>,
    #[arg(long, requires_all = ["tls_cert", "tls_key"])]
    tls_client_ca: Option<PathBuf>,
    #[arg(long, group = "tls_source", conflicts_with_all = ["tls_cert", "tls_key", "tls_client_ca"])]
    tls_from_env: bool,
}

impl RemoteOptions {
    pub(crate) fn load_tls(&self) -> Result<Option<TlsIdentity>> {
        let enabled = self.grpc_bind.is_some()
            || self.grpc_admin_bind.is_some()
            || self.rest_bind.is_some()
            || self.rest_admin_bind.is_some();
        if !enabled {
            anyhow::ensure!(
                !self.tls_from_env
                    && self.tls_cert.is_none()
                    && self.tls_key.is_none()
                    && self.tls_client_ca.is_none(),
                "TLS options require at least one remote listener"
            );
            return Ok(None);
        }
        let tls = if self.tls_from_env {
            let read = |name: &str| -> Result<Vec<u8>> {
                let value =
                    std::env::var(name).map_err(|_| anyhow!("missing or invalid {name}"))?;
                anyhow::ensure!(
                    !value.trim().is_empty() && value.len() <= 1024 * 1024,
                    "empty or oversized {name}"
                );
                Ok(value.into_bytes())
            };
            TlsIdentity {
                certificate_pem: read("HM_TLS_CERT_PEM")?,
                private_key_pem: read("HM_TLS_KEY_PEM")?,
                client_ca_pem: read("HM_TLS_CLIENT_CA_PEM")?,
            }
        } else {
            let read = |path: &Option<PathBuf>, label: &str| -> Result<Vec<u8>> {
                std::fs::read(path.as_ref().ok_or_else(|| anyhow!("missing {label}"))?)
                    .with_context(|| format!("read {label}"))
            };
            TlsIdentity {
                certificate_pem: read(&self.tls_cert, "TLS certificate")?,
                private_key_pem: read(&self.tls_key, "TLS private key")?,
                client_ca_pem: read(&self.tls_client_ca, "TLS client CA")?,
            }
        };
        Ok(Some(tls))
    }
}

async fn stopped(mut signal: watch::Receiver<bool>) {
    let _ = signal.wait_for(|value| *value).await;
}

pub(crate) async fn run(
    config: ServerConfig,
    options: RemoteOptions,
    tls: Option<TlsIdentity>,
) -> Result<Value> {
    let mut grpc = Vec::new();
    let mut rest = Vec::new();
    let config = Arc::new(config);
    if let Some(tls) = tls {
        for (address, role) in [
            (options.grpc_bind, ListenerRole::Actor),
            (options.grpc_admin_bind, ListenerRole::Admin),
        ] {
            if let Some(address) = address {
                grpc.push(
                    GrpcServer::bind(address, Gateway::new(config.clone(), role), tls.clone())
                        .await
                        .map_err(anyhow::Error::from_boxed)?,
                );
            }
        }
        for (address, role) in [
            (options.rest_bind, ListenerRole::Actor),
            (options.rest_admin_bind, ListenerRole::Admin),
        ] {
            if let Some(address) = address {
                rest.push(
                    RestServer::bind(address, Gateway::new(config.clone(), role), tls.clone())
                        .await
                        .map_err(anyhow::Error::from_boxed)?,
                );
            }
        }
    }
    let dispatcher =
        tokio::task::spawn_blocking(hm_mcp::dispatcher::McpToolDispatcher::from_env).await??;
    let uds = UdsServer::bind((*config).clone())
        .await?
        .with_tool_dispatcher(Arc::new(dispatcher));
    let (stop, signal) = watch::channel(false);
    let mut tasks: JoinSet<Result<()>> = JoinSet::new();
    for server in grpc {
        let signal = signal.clone();
        tasks.spawn(async move {
            server
                .serve_until(stopped(signal))
                .await
                .map_err(Into::into)
        });
    }
    for server in rest {
        let signal = signal.clone();
        tasks.spawn(async move {
            server
                .serve_until(stopped(signal))
                .await
                .map_err(Into::into)
        });
    }
    tasks.spawn(async move { uds.serve_until(stopped(signal)).await.map_err(Into::into) });
    let mut terminate = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
    let result = tokio::select! {
        result = tokio::signal::ctrl_c() => result.map_err(Into::into),
        _ = terminate.recv() => Ok(()),
        result = tasks.join_next() => match result {
            Some(Ok(Err(error))) => Err(error),
            Some(Err(error)) => Err(error.into()),
            _ => Err(anyhow!("server stopped unexpectedly")),
        },
    };
    let _ = stop.send(true);
    while let Some(joined) = tasks.join_next().await {
        joined??;
    }
    result?;
    Ok(json!({"ok": true, "stopped": true}))
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    #[test]
    fn remote_listeners_require_all_mutual_tls_inputs_and_keep_uds_default() {
        assert!(crate::Cli::try_parse_from(["hm", "serve", "--config", "hypermind.conf"]).is_ok());
        for listener in [
            "--grpc-bind",
            "--grpc-admin-bind",
            "--rest-bind",
            "--rest-admin-bind",
        ] {
            let base = [
                "hm",
                "--json",
                "serve",
                "--config",
                "hypermind.conf",
                listener,
                "127.0.0.1:8443",
            ];
            assert!(crate::Cli::try_parse_from(base).is_err());
            let mut complete = base.to_vec();
            complete.extend([
                "--tls-cert",
                "server.pem",
                "--tls-key",
                "server.key",
                "--tls-client-ca",
                "ca.pem",
            ]);
            assert!(crate::Cli::try_parse_from(complete).is_ok());
            let mut partial = base.to_vec();
            partial.extend(["--tls-cert", "server.pem", "--tls-key", "server.key"]);
            assert!(crate::Cli::try_parse_from(partial).is_err());
            let mut env = base.to_vec();
            env.push("--tls-from-env");
            assert!(crate::Cli::try_parse_from(&env).is_ok());
            env.extend(["--tls-key", "server.key"]);
            assert!(crate::Cli::try_parse_from(env).is_err());
        }
    }
}
