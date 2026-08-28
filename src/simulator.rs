use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::Instant;

use crate::parser::Config;

#[derive(Debug, Clone)]
pub struct Process {
    pub name: String,
    pub needs: HashMap<String, usize>,
    pub results: HashMap<String, usize>,
    pub delay: usize,
}

impl Process {
    pub fn can_start(&self, stocks: &HashMap<String, usize>) -> bool {
        for (name, qty) in &self.needs {
            match stocks.get(name) {
                Some(curr_qty) if curr_qty >= qty => {}
                _ => {
                    return false;
                }
            }
        }
        true
    }

    pub fn start(&self, stocks: &mut HashMap<String, usize>) {
        for (name, qty) in &self.needs {
            // Safe to unwrap because we assume can_start was called first
            let curr_qty = stocks.get_mut(name).unwrap();
            *curr_qty -= qty;
            if *curr_qty == 0 {
                stocks.remove(name);
            }
        }
    }

    pub fn finish(&self, stocks: &mut HashMap<String, usize>) {
        for (name, qty) in &self.results {
            let curr_qty = stocks.entry(name.clone()).or_insert(0);
            *curr_qty += qty;
        }
    }
}

struct RunningProcess {
    pub process: Process,
    pub finish_cycle: usize,
}

struct State {
    curr_cycle: usize,
    running: Vec<RunningProcess>,
    stocks: HashMap<String, usize>,
    history: Vec<String>,
}

impl State {
    fn new(stocks: HashMap<String, usize>) -> Self {
        State { curr_cycle: 0, running: Vec::new(), stocks: stocks, history: Vec::new() }
    }

    fn finish_processes(&mut self) {
        let (done, still): (Vec<RunningProcess>, Vec<RunningProcess>) = self.running
            .drain(..)
            .partition(|r| r.finish_cycle == self.curr_cycle);

        self.running = still;

        for r in done {
            r.process.finish(&mut self.stocks);
        }
    }

    fn start_processes(&mut self, processes: &[Process]) {
        for p in processes.iter() {
            while p.can_start(&self.stocks) {
                p.start(&mut self.stocks);

                self.running.push(RunningProcess {
                    process: p.clone(),
                    finish_cycle: self.curr_cycle + p.delay,
                });

                self.history.push(format!("{}:{}", self.curr_cycle, p.name));
            }
        }
    }

    fn print_result(&self) {
        println!("Main Processes :");
        for l in self.history.iter() {
            println!(" {l}");
        }
        println!("No more process doable at cycle {}", self.curr_cycle);
        println!("Stock :");

        for (name, qty) in self.stocks.iter() {
            println!(" {name} => {qty}");
        }
    }

    fn log_history(&self, filename: &str) -> Result<(), String> {
        let path = Path::new(filename).with_extension("log");

        let log_content = self.history.join("\n") + "\n";

        fs::write(path, log_content).map_err(|e| format!("Failed to write log file: {}", e))
    }
}

pub fn simulate(config: Config, filename: &str, time: usize) -> Result<(), String> {
    let mut state = State::new(config.stocks);
    let start_time = Instant::now();

    loop {
        if start_time.elapsed().as_secs() >= (time as u64) {
            println!("Timeout of {}s reached. Shutting down.", time);
            break;
        }

        state.finish_processes();
        state.start_processes(&config.processes);

        let next_cycle = state.running
            .iter()
            .map(|r| r.finish_cycle)
            .min();

        match next_cycle {
            Some(cycle) => {
                state.curr_cycle = cycle;
            }

            None => {
                break;
            }
        }
    }

    state.print_result();
    state.log_history(filename)
}
