use anyhow::Result;
use bore_cli::{client::Client, server::Server};
use clap::{error::ErrorKind, CommandFactory, Parser, Subcommand};

#[derive(Parser, Debug)]
#[clap(author, version, about)]
struct Args {
    #[clap(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug, Clone)]
enum Command {
    /// Starts a local proxy to the remote server.
    Local {
        /// The local port to expose.
        local_port: u16,

        /// The local host to expose.
        #[clap(short, long, value_name = "HOST", default_value = "localhost")]
        local_host: String,

        /// Address of the remote server to expose local ports to.
        #[clap(short, long, env = "BORE_SERVER")]
        to: String,

        /// Optional port on the remote server to select.
        #[clap(short, long, env = "BORE_FALLBACK_IP")]
        fallback_ip: Option<String>,

        /// Optional port on the remote server to select.
        #[clap(short, long, default_value_t = 0)]
        port: u16,

        /// Relentlessly try to reconnect if the connection is lost.
        #[clap(short, long, default_value_t = false)]
        relentlessly: bool,

        /// Number of retries to attempt if the connection is lost.
        #[clap(short, long, default_value_t = 0)]
        retries: u8,
        /// Optional secret for authentication.
        #[clap(short, long, env = "BORE_SECRET", hide_env_values = true)]
        secret: Option<String>,
    },

    /// Runs the remote proxy server.
    Server {
        /// Minimum accepted TCP port number.
        #[clap(long, default_value_t = 1024)]
        min_port: u16,

        /// Maximum accepted TCP port number.
        #[clap(long, default_value_t = 65535)]
        max_port: u16,

        /// Optional secret for authentication.
        #[clap(short, long, env = "BORE_SECRET", hide_env_values = true)]
        secret: Option<String>,
    },
}

#[tokio::main]
async fn run(command: Command) {
    if let Some(err) = wrap_result(command).await.err() {
        println!("Error crash occurred: {}", err)
    }
}
pub async fn start_bore_client(
    local_host: &str,
    local_port: u16,
    to: &str,
    port: u16,
    secret: Option<&str>,
) {
    let Ok(client) = Client::new(local_host, local_port, to, port, secret).await else {
        return;
    };
    let _ = client.listen().await;
}

async fn wrap_result(command: Command) -> Result<()> {
    match command {
        Command::Local {
            local_host,
            local_port,
            to,
            port,
            secret,
            relentlessly,
            retries,
            fallback_ip,
        } => {
            while relentlessly {
                start_bore_client(&local_host, local_port, &to, port, secret.as_deref()).await;

                if let Some(ref fallback_ip) = fallback_ip {
                    start_bore_client(
                        &local_host,
                        local_port,
                        fallback_ip,
                        port,
                        secret.as_deref(),
                    )
                    .await;
                }
            }
            for _ in 0..retries {
                start_bore_client(&local_host, local_port, &to, port, secret.as_deref()).await;

                if let Some(ref fallback_ip) = fallback_ip {
                    start_bore_client(
                        &local_host,
                        local_port,
                        fallback_ip,
                        port,
                        secret.as_deref(),
                    )
                    .await;
                }
            }
        }
        Command::Server {
            min_port,
            max_port,
            secret,
        } => {
            let port_range = min_port..=max_port;
            if port_range.is_empty() {
                Args::command()
                    .error(ErrorKind::InvalidValue, "port range is empty")
                    .exit();
            }
            Server::new(port_range, secret.as_deref()).listen().await?;
        }
    }

    Ok(())
}

fn main() {
    tracing_subscriber::fmt::init();
    run(Args::parse().command);
}
