#![forbid(unsafe_code)]

use hm_eval::slice1;

#[tokio::main]
async fn main() {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let result = match arguments.as_slice() {
        [command, slice] if command == "gate" && slice == "slice1" => slice1::gate(None).await,
        [command, slice, flag, reference]
            if command == "gate" && slice == "slice1" && flag == "--compare" =>
        {
            slice1::gate(Some(reference)).await
        }
        _ => {
            eprintln!("usage: hm-eval gate slice1 [--compare REF]");
            std::process::exit(2);
        }
    };
    if let Err(error) = result {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
