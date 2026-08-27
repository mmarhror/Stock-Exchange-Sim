use crate::parser::Process;

pub struct RunningProcess {
    pub end: usize,
    pub process: Process,
}
