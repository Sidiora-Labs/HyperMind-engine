#![forbid(unsafe_code)]

use hm_eval::{bench, slice1, slice2, slice3, slice4, slice5, slice6, slice7, suites};

#[allow(clippy::too_many_lines)]
#[tokio::main]
async fn main() {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let result = match arguments.as_slice() {
        [command] if command == "docs-gate" => hm_eval::docs::gate(),
        [command, flag] if command == "budget" && flag == "--confirm-upstream-rates" => {
            (|| -> Result<(), Box<dyn std::error::Error>> {
                let gateway = bench::gateway::Gateway::open(
                    "eval/cache/slice7-gateway",
                    false,
                    bench::gateway::MAXIMUM_BUDGET_MICROUSD,
                )
                .map_err(|error| error as Box<dyn std::error::Error>)?;
                let summary = gateway
                    .confirm_upstream_rates()
                    .map_err(|error| error as Box<dyn std::error::Error>)?;
                println!("{}", serde_json::to_string_pretty(&summary)?);
                Ok(())
            })()
        }
        [command, name, id, prediction, flag]
            if command == "rejudge" && name == "longmemeval" && flag == "--live" =>
        {
            bench::execution::diagnostic_judge(id, prediction, true)
                .await
                .map_err(|error| error as Box<dyn std::error::Error>)
        }
        [command, name, id, prediction] if command == "rejudge" && name == "longmemeval" => {
            bench::execution::diagnostic_judge(id, prediction, false)
                .await
                .map_err(|error| error as Box<dyn std::error::Error>)
        }
        [command, name, selector, ids, flag]
            if command == "diagnose"
                && name == "longmemeval"
                && selector == "--questions"
                && flag == "--live" =>
        {
            bench::execution::diagnostic_longmemeval(ids, true)
                .await
                .map_err(|error| error as Box<dyn std::error::Error>)
        }
        [command, name, selector, ids]
            if command == "diagnose" && name == "longmemeval" && selector == "--questions" =>
        {
            bench::execution::diagnostic_longmemeval(ids, false)
                .await
                .map_err(|error| error as Box<dyn std::error::Error>)
        }
        [command] if command == "provider-check" => bench::execution::provider_check()
            .await
            .map_err(|error| error as Box<dyn std::error::Error>),
        [command, name, flag] if command == "bench" && flag == "--live" => {
            bench::execution::benchmark(name, true)
                .await
                .map_err(|error| error as Box<dyn std::error::Error>)
        }
        [command, name] if command == "bench" => bench::execution::benchmark(name, false)
            .await
            .map_err(|error| error as Box<dyn std::error::Error>),
        [command, slice] if command == "gate" && slice == "slice1" => slice1::gate(None).await,
        [command, slice] if command == "gate" && slice == "slice2" => slice2::gate(None).await,
        [command, slice] if command == "gate" && slice == "slice3" => slice3::gate(None),
        [command, slice] if command == "gate" && slice == "slice4" => slice4::gate(None),
        [command, slice] if command == "gate" && slice == "slice5" => slice5::gate(None).await,
        [command, slice] if command == "gate" && slice == "slice6" => slice6::gate(None).await,
        [command, slice] if command == "gate" && slice == "slice7" => slice7::gate(None).await,
        [command, slice, flag, reference]
            if command == "gate" && slice == "slice7" && flag == "--compare" =>
        {
            slice7::gate(Some(reference)).await
        }
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
                "usage: hm-eval gate <slice1|slice2|slice3|slice4|slice5|slice6|slice7> [--compare REF]\n       hm-eval bench <longmemeval|locomo> [--live]\n       hm-eval diagnose longmemeval --questions ID,ID [--live]\n       hm-eval rejudge longmemeval ID PREDICTION_PATH [--live]\n       hm-eval provider-check\n       hm-eval budget --confirm-upstream-rates"
            );
            std::process::exit(2);
        }
    };
    if let Err(error) = result {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
