use core::fmt;

#[derive(Debug)]
pub struct TimeoutError {
    pub operation: String,
    pub duration: std::time::Duration,
}

impl fmt::Display for TimeoutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Operation '{}', timed out after {:?}",
            self.operation, self.duration
        )
    }
}
