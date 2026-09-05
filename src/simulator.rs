use std::os::macos::raw::stat;
use std::{ collections::HashMap, time::Duration };
use std::fs;
use std::path::Path;
use std::time::Instant;

use crate::parser::{ Config, Process };

#[derive(Clone)]
pub struct RunningProcess {
    pub process: Process,
    pub finish_cycle: usize,
}

#[derive(Clone)]
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

    fn get_next_cycle(&self) -> Option<usize> {
        self.running
            .iter()
            .map(|r| r.finish_cycle)
            .min()
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

    pub fn advance_to(&mut self, cycle: usize) -> Result<(), String> {
        if cycle < self.curr_cycle {
            return Err(
                format!(
                    "Invalid timestamp: cannot go backwards from cycle {} to {}",
                    self.curr_cycle,
                    cycle
                )
            );
        }

        while self.curr_cycle < cycle {
            self.curr_cycle += 1;
            self.finish_processes();
        }

        Ok(())
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

    pub fn finish_all(&mut self) {
        while
            let Some(next_cycle) = self.running
                .iter()
                .map(|r| r.finish_cycle)
                .min()
        {
            let _ = self.advance_to(next_cycle);
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

fn search(state: &State, processes: &[Process], deadline: Instant, best: &mut Option<State>) {
    if Instant::now() >= deadline {
        return;
    }

    let any_startable: bool = processes.iter().any(|p| p.can_start(&state.stocks));
    let any_running: bool = !state.running.is_empty();

    if !any_startable && !any_running {
        *best = Some(state.clone());
        return;
    }

    let mut child = state.clone();

    for process in processes {
        if process.can_start(&state.stocks) {
            let _ = child.start_by_name(&process.name, processes);
            search(&child, processes, deadline, best);
        }
    }

    if any_running {
        let next_cycle = child.get_next_cycle().unwrap();
        let _ = child.advance_to(next_cycle);
        search(&child, processes, deadline, best);
    }
}

pub fn simulate(config: Config, filename: &str, time: usize) {
    let deadline = Instant::now() + Duration::from_secs(time as u64);
    let initial_state = State::new(config.stocks.clone());
    let mut best: Option<State> = None;

    search(&initial_state, &config.processes, deadline, &mut best);

    match best {
        Some(final_state) => {
            final_state.print_result();
            final_state.log_history(filename);
        }
        None => {
            println!("No complete schedule found within {}s.", time);
        }
    }
}
