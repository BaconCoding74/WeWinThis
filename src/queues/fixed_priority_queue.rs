use crate::common::job;

#[derive(Debug, Clone)]
pub struct FixedPriorityQueue<const N: usize> {
    data: [Option<job::Job>; N],
    len: usize,
}

impl<const N: usize> FixedPriorityQueue<N> {
    pub fn new() -> Self {
        assert!(N > 0, "capacity must be > 0");
        Self {
            data: [None; N],
            len: 0,
        }
    }

    pub const fn capacity(&self) -> usize {
        N
    }

    pub const fn len(&self) -> usize {
        self.len
    }

    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub const fn is_full(&self) -> bool {
        self.len == N
    }

    fn higher_priority(a: job::Job, b: job::Job) -> bool {
        if a.priority != b.priority {
            return a.priority < b.priority;
        }

        if a.release_tick != b.release_tick {
            return a.release_tick < b.release_tick;
        }

        a.seq < b.seq
    }

    pub fn push(&mut self, job: job::Job) -> Result<(), job::Job> {
        if self.is_full() {
            return Err(job);
        }

        let mut insert_at = self.len;

        while insert_at > 0 {
            let prev = self.data[insert_at - 1].unwrap();

            if Self::higher_priority(job, prev) {
                self.data[insert_at] = self.data[insert_at - 1];
                insert_at -= 1;
            } else {
                break;
            }
        }

        self.data[insert_at] = Some(job);
        self.len += 1;
        Ok(())
    }

    pub fn pop(&mut self) -> Option<job::Job> {
        if self.is_empty() {
            return None;
        }

        let best = self.data[0].take();

        let mut i = 1;
        while i < self.len {
            self.data[i - 1] = self.data[i];
            i += 1;
        }

        self.data[self.len - 1] = None;
        self.len -= 1;

        best
    }

    pub const fn peek(&self) -> Option<job::Job> {
        if self.is_empty() {
            None
        } else {
            self.data[0]
        }
    }

    pub fn clear(&mut self) {
        while self.pop().is_some() {}
    }
}