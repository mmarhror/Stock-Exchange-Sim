//! Checker Tool
//!
//! This binary validates whether a generated schedule log is executionally correct.
//! It replays each action from the log chronologically against the rules of the
//! simulation, ensuring that:
//! 1. Timestamps never go backward.
//! 2. Processes are known and exist in the configuration.
//! 3. All process starts are affordable based on available stocks at that cycle.
//!
//! # Verification Flow
//!
//! For each entry `cycle:process_name` in the log:
//! 1. **Advance Time:** Jump to `cycle`. This finishes any running processes that complete
//!    before or at this cycle, harvesting their results into stocks.
//! 2. **Start Process:** Verify the process can start, consume its requirements, and add
//!    it to the active running list.

use std::env;
use std::process;

use stock_exchange::parser;
use stock_exchange::simulator::State;

fn main() {
    let args: Vec<String> = env::args().collect();

    // 1. Verify CLI arguments
    if args.len() < 3 {
        eprintln!("Usage: checker <config_file> <log_file>");
        process::exit(1);
    }

    let config_file = &args[1];
    let log_file = &args[2];

    // 2. Parse configuration file
    let config = match parser::parse_file(config_file) {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("Config error: {}", e);
            process::exit(1);
        }
    };

    // 3. Parse action log file
    let actions = match parser::parse_log(log_file) {
        Ok(acts) => acts,
        Err(e) => {
            eprintln!("Log error: {}", e);
            process::exit(1);
        }
    };

    // 4. Initialize replay state with starting config stocks
    let mut state = State::new(config.stocks);

    // 5. Replay and validate each log action chronologically
    for (log_cycle, proc_name) in actions {
        let line = format!("{log_cycle}:{proc_name}");

        println!("Evaluating: {}", line);

        // Advance the clock to the action's cycle.
        // This processes all intermediate completed tasks, populating stocks.
        if let Err(e) = state.advance_to(log_cycle) {
            println!("Error detected\nat {} {}", line, e);
            process::exit(1);
        }

        // Attempt to start the process at the current cycle.
        // Fails if the process is unknown or stocks are insufficient.
        if let Err(e) = state.start_by_name(&proc_name, &config.processes) {
            println!("Error detected\nat {} {}", line, e);
            process::exit(1);
        };
    }

    // 6. Finish all running tasks left over at the end of the log
    state.finish_all_remaining();

    // 7. Success confirmation
    println!("Trace completed, no error detected.");
}
