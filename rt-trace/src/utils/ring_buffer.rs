/// A fixed-capacity ring buffer (circular buffer) that overwrites the oldest elements when full.
///
/// Internally uses a `Vec<T>` and a head index to track where to overwrite next.
pub struct RingBuffer<T> {
    inner: Vec<T>,
    capacity: usize,
    head: usize,
}

impl<T> RingBuffer<T> {
    /// Creates a new `RingBuffer` with the given capacity.
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0, "Capacity must be greater than zero");
        RingBuffer {
            inner: Vec::with_capacity(capacity),
            capacity,
            head: 0,
        }
    }

    /// Appends all items from `items` into the buffer.
    ///
    /// If the buffer is not yet full, this will push new elements.
    /// Once full, further elements overwrite the oldest entries in FIFO order.
    pub fn push_overwrite(&mut self, item: T) {
        if self.inner.len() < self.capacity {
            // Buffer not yet full: push normally
            self.inner.push(item);
        } else {
            // Buffer full: overwrite at head index
            self.inner[self.head] = item;
            // Advance head wrap-around
            self.head = (self.head + 1) % self.capacity;
        }
    }

    /// Drains all elements, returning them in FIFO order, and resets the buffer to empty.
    pub fn drain(&mut self) -> Vec<T> {
        let len = self.inner.len();
        let mut out = Vec::with_capacity(len);
        if self.head < len {
            // Drain from head to end
            out.extend(self.inner.drain(self.head..));
            // Drain from start to head
            out.extend(self.inner.drain(0..self.head));
        }
        // Reset head
        self.head = 0;
        out
    }
}

// TODO: enable
// #[cfg(test)]
// mod tests {
//     use super::RingBuffer;

//     #[test]
//     fn basic_enqueue_and_overwrite() {
//         let mut buf = RingBuffer::new(3);
//         buf.push_overwrite(1);
//         buf.push_overwrite(2);
//         assert_eq!(buf.inner.iter().cloned().collect::<Vec<_>>(), vec![1, 2]);
//         // Fill to capacity
//         buf.push_overwrite(3);
//         assert_eq!(buf.inner.iter().cloned().collect::<Vec<_>>(), vec![1, 2, 3]);
//         // Overwrite oldest
//         buf.push_overwrite(4);
//         assert_eq!(buf.inner.iter().cloned().collect::<Vec<_>>(), vec![2, 3, 4]);
//         // Multiple items (push individually)
//         buf.push_overwrite(5);
//         buf.push_overwrite(6);
//         buf.push_overwrite(7);
//         // Should end up with last 3 in order: 5,6,7 overwrite
//         assert_eq!(buf.inner.iter().cloned().collect::<Vec<_>>(), vec![5, 6, 7]);
//     }
// }
