use std::ops::{Generator, GeneratorState};
use std::pin::Pin;

pub struct GenIter<G> {
    gen: Pin<Box<G>>,
    done: bool
}

impl<G> GenIter<G> {
    pub fn new(gen: G) -> Self {
        GenIter {
            gen: Box::pin(gen),
            done: false
        }
    }
}

impl<G> Iterator for GenIter<G> where G: Generator<Return=()> {
    type Item = G::Yield;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        match self.gen.as_mut().resume(()) {
            GeneratorState::Yielded(item) => Some(item),
            GeneratorState::Complete(()) => {
                self.done = true;
                None
            }
        }
    }
}