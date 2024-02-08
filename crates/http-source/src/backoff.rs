use std::time::Duration;

pub(crate) enum Backoff {
    Fib(u64, u64),
}

impl Backoff {
    pub fn new() -> Self {
        Backoff::Fib(1, 1)
    }

    pub fn next(&mut self) -> Duration {
        match self {
            Backoff::Fib(a, b) => {
                let next = *a + *b;
                *self = Backoff::Fib(*b, next);

                std::time::Duration::from_millis(10 * next)
            }
        }
    }
}
