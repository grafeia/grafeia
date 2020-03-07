use crate::layout::*;

#[derive(Debug)]
pub struct Writer {
    state:      Glue,
    stream:     StreamVec,
    branch_stack: BranchStack,
}

#[derive(Debug)]
enum BranchState {
    // initially at Pre
    Pre,
    // then we get called before A gets pushed
    A { entry_pos: usize },
    // and again after A was pushed and before B is pushed
    B { exit_pos: usize },
    // and again after B was pushed
    Post
}
impl BranchState {
    fn step(&mut self, stream: &mut StreamVec) {
        let pos = stream.len();
        *self = match *self {
            BranchState::Pre => {
                stream.push(Entry::BranchEntry(0));

                BranchState::A { entry_pos: pos }
            },
            BranchState::A { entry_pos } => {
                // figure out the length of a
                stream.push(Entry::BranchEntry(0));
                
                let len_a = pos - entry_pos;
                stream.set(entry_pos, Entry::BranchEntry(len_a));

                BranchState::B { exit_pos: pos }
            }
            BranchState::B { exit_pos } => {
                let len_b = pos - exit_pos - 1;
                stream.set(exit_pos, Entry::BranchExit(len_b));

                BranchState::Post
            },
            BranchState::Post => panic!()
        };
    }
}

#[derive(Debug)]
struct Branch {
    size: usize,
    state: BranchState,
}

#[derive(Default, Debug)]
struct BranchStack {
    items: Vec<Branch>
}
impl BranchStack {
    fn init(&mut self, n: usize) {
        self.items.clear();
        if n > 1 {
            self.items.push(Branch {
                size: n,
                state: BranchState::Pre
            });
        }
    }
    fn step(&mut self, stream: &mut StreamVec) {
        while self.items.len() > 0 {
            let last = self.items.last_mut().unwrap();
            if let BranchState::B { .. } = last.state {
                last.state.step(stream);
                self.items.pop();
                continue;
            }

            match last {
                &mut Branch { size: 2, ref mut state } => {
                    state.step(stream);
                    return;
                },
                &mut Branch { size, ref mut state } if size > 2 => {
                    state.step(stream);
                    let half = size / 2;
                    let branch_size = match *state {
                        BranchState::A { .. } => half,
                        BranchState::B { .. } => size - half,
                        _ => unreachable!()
                    };
                    if branch_size == 1 {
                        // good to go
                        return;
                    }
                    self.items.push(Branch {
                        size: branch_size,
                        state: BranchState::Pre
                    });
                }
                b => panic!("{:?}", b)
            }
        }
    }
    fn finish(&mut self, stream: &mut StreamVec) {
        self.step(stream);
        assert_eq!(self.items.len(), 0);
    }
}

impl Writer {
    pub fn new() -> Writer {
        Writer {
            state:  Glue::None,
            stream: StreamVec::new(),
            branch_stack: BranchStack::default(),
        }
    }
    pub fn with_stream(stream: StreamVec) -> Writer {
        Writer {
            state:  Glue::None,
            stream,
            branch_stack: BranchStack::default(),
        }
    }
    pub fn finish(mut self) -> StreamVec {
        self.write_glue(Glue::any());
        self.stream
    }
    pub fn clear(&mut self) {
        self.state = Glue::None;
        self.stream.0.clear();
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

    // much faster variant, but expects exactly count branches (calls to f)
    pub fn branch2(&mut self, count: usize, f: impl FnOnce(&mut Gen2)) {
        self.branch_stack.init(count);
        f(&mut Gen2 {
            writer: self,
            count,
            at: 0
        });
        self.branch_stack.finish(&mut self.stream);
    }
}

#[derive(Debug)]
pub struct Gen2<'a> {
    writer: &'a mut Writer,
    count: usize,
    at: usize,
}
impl<'a> Gen2<'a> {
    pub fn add(&mut self, f: impl FnOnce(&mut Writer)) {
        assert!(self.at < self.count);
        self.writer.branch_stack.step(&mut self.writer.stream);
        f(self.writer);
        self.at += 1;
    }
}
