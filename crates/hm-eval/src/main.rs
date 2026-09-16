#![forbid(unsafe_code)]

use hm_eval::{slice1, slice2, slice3, slice4, slice5, slice6, suites};

#[tokio::main]
async fn main() {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let result = match arguments.as_slice() {
        [command, slice] if command == "gate" && slice == "slice1" => slice1::gate(None).await,
        [command, slice] if command == "gate" && slice == "slice2" => slice2::gate(None).await,
        [command, slice] if command == "gate" && slice == "slice3" => slice3::gate(None),
        [command, slice] if command == "gate" && slice == "slice4" => slice4::gate(None),
        [command, slice] if command == "gate" && slice == "slice5" => slice5::gate(None).await,
        [command, slice] if command == "gate" && slice == "slice6" => slice6::gate(None).await,
        [command, slice, flag, reference]
            if command == "gate" && slice == "slice1" && flag == "--compare" =>
        {
            slice1::gate(Some(reference)).await
        }
        [command, slice, flag, reference]
            if command == "gate" && slice == "slice2" && flag == "--compare" =>
        {
            slice2::gate(Some(reference)).await
        }
        [command, slice, flag, reference]
            if command == "gate" && slice == "slice3" && flag == "--compare" =>
        {
            slice3::gate(Some(reference))
        }
        [command, slice, flag, reference]
            if command == "gate" && slice == "slice4" && flag == "--compare" =>
        {
            slice4::gate(Some(reference))
        }
        [command, slice, flag, reference]
            if command == "gate" && slice == "slice5" && flag == "--compare" =>
        {
            slice5::gate(Some(reference)).await
        }
        [command, slice, flag, reference]
            if command == "gate" && slice == "slice6" && flag == "--compare" =>
        {
            slice6::gate(Some(reference)).await
        }
        [command, directory, marker, trial] if command == "__continuity_child" => {
            match trial.parse::<u8>() {
                Ok(trial) => suites::continuity::child(
                    std::path::Path::new(directory),
                    std::path::Path::new(marker),
                    trial,
                )
                .await
                .map_err(Into::into),
                Err(error) => Err(error.into()),
            }
        }
        [command, directory, marker, mode] if command == "__generation_child" => {
            suites::generations::child(
                std::path::Path::new(directory),
                std::path::Path::new(marker),
                mode,
            )
            .await
        }
        _ => {
            eprintln!(
                "usage: hm-eval gate <slice1|slice2|slice3|slice4|slice5|slice6> [--compare REF]"
            );
            std::process::exit(2);
        }
    };
    if let Err(error) = result {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
