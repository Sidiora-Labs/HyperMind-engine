#![forbid(unsafe_code)]

#[tokio::main]
async fn main() {
    if let Err(error) = hm_cli::run(std::env::args_os()).await {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
