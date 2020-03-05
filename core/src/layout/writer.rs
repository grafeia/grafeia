use crate::layout::*;
use std::iter::Extend;

pub struct BranchGenerator<'a> {
    parent: &'a Writer,
    branches: Vec<(StreamVec, Glue)>
}
impl<'a> BranchGenerator<'a> {
    pub fn add(&mut self, f: impl FnOnce(&mut Writer)) {
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
                (&Entry::Space(a_measure, a_line, a_col), &Entry::Space(b_measure, b_line, b_col)) =>
                    (a_measure, a_line, a_col) == (b_measure, b_line, b_col),
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
            Glue::Column => self.stream.push(Entry::Column),
            Glue::Newline { fill, height, column_break } => {
                self.stream.push(Entry::Linebreak(fill, height, column_break));
            },
            Glue::Space { measure, line_break, column_break }
             => self.stream.push(Entry::Space(measure, line_break, column_break)),
            Glue::None => ()
        }
    }
    
    fn push(&mut self, left: Glue, right: Glue, entry: Entry) {
        self.write_glue(left);
        self.stream.push(entry);
        self.state = right;
    }

    pub fn item(&mut self, left: Glue, right: Glue, measure: ItemMeasure, item: RenderItem, tag: Tag) {
        self.push(left, right, Entry::Item(measure, item, tag));
    }
    pub fn space(&mut self, left: Glue, right: Glue, measure: FlexMeasure, line_break: Option<f32>, column_break: Option<f32>) {
        self.push(left, right, Entry::Space(measure, line_break, column_break));
    }
    pub fn set_width(&mut self, indent: Length, width: Length) {
        self.stream.push(Entry::SetWidth(indent, width));
    }

    #[inline(always)]
    pub fn promote(&mut self, glue: Glue) {
        self.state |= glue;
    }

    pub fn branch(&mut self, f: impl FnOnce(&mut BranchGenerator))
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
 
