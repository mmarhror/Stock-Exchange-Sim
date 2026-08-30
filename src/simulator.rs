use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::Instant;

use crate::parser::{ Config, Process };

pub struct RunningProcess {
    pub process: Process,
    pub finish_cycle: usize,
}

pub struct State {
    pub curr_cycle: usize,
    pub running: Vec<RunningProcess>,
    pub stocks: HashMap<String, usize>,
    pub history: Vec<String>,
}

impl State {
    pub fn new(stocks: HashMap<String, usize>) -> Self {
        State { curr_cycle: 0, running: Vec::new(), stocks, history: Vec::new() }
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

    pub fn advance_to(&mut self, cycle: usize) {
        while self.curr_cycle < cycle {
            self.curr_cycle += 1;
            self.finish_processes();
        }
    }

    pub fn start_by_name(&mut self, name: &str, processes: &[Process]) -> Result<(), String> {
        let process = processes
            .iter()
            .find(|p| p.name == name)
            .ok_or_else(|| format!("unknown process: {}", name))?;

        if !process.can_start(&self.stocks) {
            return Err("stock insufficient".to_string());
        }

        process.start(&mut self.stocks);
        self.running.push(RunningProcess {
            process: process.clone(),
            finish_cycle: self.curr_cycle + process.delay,
        });

        Ok(())
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
