use crate::layout::*;
use std::iter::Extend;

pub struct BranchGenerator<'a> {
    parent: &'a Writer,
    branches: Vec<(StreamVec, Glue)>
}
impl<'a> BranchGenerator<'a> {
    pub fn add(&mut self, mut f: impl FnMut(&mut Writer)) {
        let mut w = self.parent.dup();
        f(&mut w);
        self.branches.push((w.stream, w.state));
    }
}
pub struct Writer {
    state:      Glue,
    stream:     StreamVec,
}

impl StreamVec {
    // careful with the arguments.. they all have the same type!
    fn merge(&mut self, StreamVec(mut a): StreamVec, StreamVec(mut b): StreamVec) {
        let out = &mut self.0;

        if a.len() == 0 {
            out.extend(b);
        } else if b.len() == 0 {
            out.extend(a);
        } else {
            let equal_end = match (a.last().unwrap(), b.last().unwrap()) {
                (&Entry::Space(a_break, a_measure), &Entry::Space(b_break, b_measure)) =>
                    (a_break == b_break) && (a_measure == b_measure),
                _ => false
            };
            
            let end_sym = if equal_end {
                a.pop();
                b.pop()
            } else {
                None
            };

            out.push(Entry::BranchEntry(b.len() + 1));
            out.extend(b);
            out.push(Entry::BranchExit(a.len()));
            out.extend(a);
            
            if let Some(end) = end_sym {
                out.push(end);
            }
        }
    }
}
impl Writer {
    pub fn new() -> Writer {
        Writer {
            state:  Glue::None,
            stream: StreamVec::new(),
        }
    }
    fn dup(&self) -> Writer {
        Writer {
            stream: StreamVec::new(),
            ..      *self
        }
    }
    
    pub fn finish(mut self) -> StreamVec {
        self.write_glue(Glue::any());
        self.stream
    }
    
    fn push_branch<I>(&mut self, mut ways: I) where I: Iterator<Item=StreamVec> {
        if let Some(default) = ways.next() {
            let mut others: Vec<StreamVec> = ways.collect();
            
            if others.len() == 0 {
                self.stream.0.extend(default.0);
                return;
            }
            
            while others.len() > 1 {
                for n in 0 .. others.len() / 2 {
                    use std::mem;
                    // TODO use with_capacity
                    let mut merged = StreamVec::new();
                    let mut tmp = StreamVec::new();
                    
                    mem::swap(&mut tmp, others.get_mut(n).unwrap());
                    merged.merge(tmp, others.pop().unwrap());
                    others[n] = merged;
                }
            }
            self.stream.merge(default, others.pop().unwrap());
        }
    }
    
    #[inline(always)]
    fn write_glue(&mut self, left: Glue) {
        match self.state | left {
            Glue::Newline { fill: f } => {
                self.stream.push(Entry::Linebreak(f));
            },
            Glue::Space { breaking, measure }
             => self.stream.push(Entry::Space(breaking, measure)),
            Glue::None => ()
        }
    }
    
    fn push(&mut self, left: Glue, right: Glue, entry: Entry) {
        self.write_glue(left);
        self.stream.push(entry);
        self.state = right;
    }

    pub fn word(&mut self, left: Glue, right: Glue, key: WordKey, measure: FlexMeasure, font: Font, tag: Tag) {
        self.push(left, right, Entry::Word(key, measure, font, tag));
    }
    pub fn space(&mut self, left: Glue, right: Glue, measure: FlexMeasure, breaking: bool) {
        self.push(left, right, Entry::Space(breaking, measure));
    }
    pub fn object(&mut self, left: Glue, right: Glue, key: ObjectKey, measure: FlexMeasure, tag: Tag) {
        self.push(left, right, Entry::Object(key, measure, tag));
    }
    
    #[inline(always)]
    pub fn promote(&mut self, glue: Glue) {
        self.state |= glue;
    }

    pub fn branch(&mut self, mut f: impl FnMut(&mut BranchGenerator))
    {
        let mut branches = {
            let mut gen = BranchGenerator {
                parent:     self,
                branches:   Vec::new()
            };
            f(&mut gen);
        
            gen.branches
        };
        let mut glue = Glue::any();
        self.push_branch(branches.drain(..).map(|(v, s)| {
            glue |= s;
            v
        }));
        self.state = glue;
        // FIXME
        //self.state = right;
    }
}
 
