use std::env;
use std::process;

use stock_exchange::parser;
use stock_exchange::simulator::State;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 3 {
        eprintln!("Usage: checker <config_file> <log_file>");
        process::exit(1);
    }

    let config_file = &args[1];
    let log_file = &args[2];

    let config = match parser::parse_file(config_file) {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("Config error: {}", e);
            process::exit(1);
        }
    };

    let actions = match parser::parse_log(log_file) {
        Ok(acts) => acts,
        Err(e) => {
            eprintln!("Log error: {}", e);
            process::exit(1);
        }
    };

    let mut state = State::new(config.stocks);

    for (log_cycle, proc_name) in actions {
        let line = format!("{log_cycle}:{proc_name}");

        println!("Evaluating: {}", line);

        if let Err(e) = state.advance_to(log_cycle) {
            println!("Error detected\nat {} {}", line, e);
            process::exit(1);
        }

        if let Err(e) = state.start_by_name(&proc_name, &config.processes) {
            println!("Error detected\nat {} {}", line, e);
            process::exit(1);
        };
    }

    state.finish_all();

    println!("Trace completed, no error detected.");
}
