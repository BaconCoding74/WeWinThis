#[derive(Debug, Clone)]
pub struct BoundedBuffer<T: Copy, const N: usize> {
    buf: [Option<T>; N],
    head: usize,
    tail: usize,
    len: usize,
}

impl <T: Copy, const N: usize> BoundedBuffer<T, N> {
    pub fn new() -> Self {
        assert!(N > 0, "Capacity must > 0");
        Self {
            buf: [None; N],
            head: 0,
            tail: 0,
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

    pub fn push(&mut self, item: T) -> Result<(), T> {
        if self.is_full() {
            return Err(item);
        }

        self.buf[self.tail] = Some(item);
        self.tail = (self.tail + 1) % N;
        self.len += 1;
        Ok(())
    }
    
    pub fn pop(&mut self) -> Option<T> {
        if self.is_empty() {
            return None;
        }
        
        let item = self.buf[self.head].take();
        self.head = (self.head + 1) % N;
        self.len -= 1;
        item
    }
    
    pub const fn peek(&self) -> Option<T> {
        if self.is_empty() {
            return None;
        }
        else {
            self.buf[self.head]
        }
    }
    
    pub fn clear(&mut self) {
        while self.pop().is_some() {}
    }
}