pub struct BatchIterator<T: Iterator, U: Iterator> {
    items: T,
    come_with_batch: U,
    batch: usize,
    current_batch: usize,
}

impl<T: Iterator, U: Iterator> BatchIterator<T, U> {
    pub fn new(items: T, come_with_batch: U, batch: usize) -> Self {
        Self {
            items,
            batch,
            current_batch: 0,
            come_with_batch,
        }
    }

    pub fn current_batch(&self) -> usize {
        self.current_batch
    }
}

impl<T: Iterator, U: Iterator> Iterator for BatchIterator<T, U> {
    type Item = (U::Item, Vec<T::Item>);

    fn next(&mut self) -> Option<Self::Item> {
        let mut temp_vec = vec![];
        for i in 0..self.batch {
            if let Some(item) = self.items.next() {
                temp_vec.push(item);
            } else if i == 0 {
                return None;
            }
        }
        self.current_batch += 1;
        Some((self.come_with_batch.next().unwrap(), temp_vec))
    }
}
