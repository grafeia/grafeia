use super::{Entry, StreamVec, FlexMeasure, Item};
//use layout::style::{Style};
use crate::units::Length;
use std::fmt::{self, Debug};
use crate::content::{Font, Tag};

#[derive(Copy, Clone, Debug, Default)]
struct LineBreak {
    prev:   usize, // index to previous line-break
    path:   u64, // one bit for each branch taken (1) or not (0)
    factor: f32,
    score:  f32,
    height: Length,
}

#[derive(Copy, Clone, Debug, Default)]
struct ColumnBreak {
    prev:   usize, // index to previous column-break
    score:  f32,
}
    
#[derive(Copy, Clone, Debug, Default)]
struct Break {
    line:   LineBreak,
    column: Option<ColumnBreak>
}

#[derive(Debug)]
pub struct ParagraphStyle {
    pub font: Font,
    pub leading: f32,
    pub par_indent: f32
}

pub struct ParagraphLayout<'a> {
    items:      &'a [Entry],
    nodes:      Vec<Option<LineBreak>>,
    width:      Length,
    last:       usize
}
pub struct ColumnLayout<'a> {
    para:       ParagraphLayout<'a>,
    nodes_col:  Vec<Option<ColumnBreak>>,
    height:     Length
}
impl<'a> Debug for ColumnLayout<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ColumnLayout")
    }
}
impl<'a> Debug for ParagraphLayout<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ParagraphLayout")
    }
}

struct Context {
    measure:    FlexMeasure,
    path:       u64,    // one bit for each branch on this line
    begin:      usize,  // begin of line or branch
    pos:        usize,  // calculation starts here
    score:      f32,    // score at pos
    branches:   u8,     // number of branches so far (<= 64)
    punctuaton: FlexMeasure
}
impl Context {
    fn new(start: usize, score: f32) -> Context {
        Context {
            measure:    FlexMeasure::zero(),
            path:       0,
            begin:      start,
            pos:        start,
            branches:   0,
            score:      score,
            punctuaton: FlexMeasure::zero()
        }
    }
    fn add_word(&mut self, measure: FlexMeasure) {
        self.measure += self.punctuaton + measure;
        self.punctuaton = FlexMeasure::zero();
    }
    fn add_punctuation(&mut self, measure: FlexMeasure) {
        self.punctuaton = measure;
    }
    fn line(&self) -> FlexMeasure {
        self.measure + self.punctuaton * 0.5
    }
    fn fill(&mut self, width: Length) {
        self.measure = self.line();
        self.measure.extend(width);
        self.punctuaton = FlexMeasure::zero();
    }
}

impl<'a> ParagraphLayout<'a> {
    pub fn new(items: &'a StreamVec, width: Length) -> ParagraphLayout<'a> {
        let limit = items.0.len();
        let mut nodes = vec![None; limit+1];
        nodes[0] = Some(LineBreak::default());

        let mut layout = ParagraphLayout {
            nodes,
            items: &items.0,
            width,
            last: 0
        };
        layout.run();
        layout
    }
    fn run(&mut self) {
        let mut last = 0;
        for start in 0 .. self.items.len() {
            match self.nodes[start] {
                Some(b) => {
                    last = self.complete_line(
                        start,
                        Context::new(start, b.score)
                    );
                },
                None => {}
            }
        }

        if self.nodes[last].is_none() {
            for i in 0 .. last {
                println!("{:3} {:?}", i, self.items[i]);
                if let Some(b) = self.nodes[i] {
                    println!("     {:?}", b);
                }
            }
        }

        self.last = last;
    }

    fn complete_line(&mut self, start: usize, mut c: Context) -> usize {
        let mut last = c.begin;
        
        while c.pos < self.items.len() {
            let n = c.pos;
            match self.items[n] {
                Entry::Word(_, m, _, _) => c.add_word(m),
                Entry::Object(_, m, _) => c.add_word(m),
                Entry::Punctuation(_, m, _, _) => c.add_punctuation(m),
                Entry::Space(breaking, s) => {
                    if breaking {
                        // breaking case:
                        // width is not added yet!
                        self.maybe_update(&c, n+1);
                        last = n+1;
                    }
                    
                    // add width now.
                    c.measure += s;
                }
                Entry::Linebreak(fill) => {
                    if fill {
                        c.fill(self.width);
                    }
                    
                    self.maybe_update(&c, n+1);
                    last = n+1;
                    break;
                },
                Entry::BranchEntry(len) => {
                    // b
                    let b_last = self.complete_line(
                        start,
                        Context {
                            pos:        n + 1,
                            path:       c.path | (1 << c.branches),
                            branches:   c.branches + 1,
                            ..          c
                        }
                    );
                    if b_last > last {
                        last = b_last;
                    }
                    
                    // a follows here
                    c.pos += len;
                    c.branches += 1;
                },
                Entry::BranchExit(skip) => {
                    c.pos += skip;
                }
                _ => {}
            }
            
            if c.measure.shrink > self.width {
                break; // too full
            }
            
            c.pos += 1;
        }
        
        last
    }

    fn maybe_update(&mut self, c: &Context, n: usize) {
        let (factor, score) = match c.line().factor(self.width) {
            Some(factor) => (factor, -factor * factor),
            None => (1.0, -1000.)
        };

        let break_score = c.score + score;
        let break_point = LineBreak {
            prev:   c.begin,
            path:   c.path,
            factor: factor,
            score:  break_score,
            height: c.measure.height
        };
        self.nodes[n] = Some(match self.nodes[n] {
            Some(line) if break_score <= line.score => line,
            _ => break_point
        });
    }
    pub fn lines<'l>(&'l self) -> Column<'l, 'a> {
        Column::new(0, self.last, self)
    }
}
impl<'a> ColumnLayout<'a>  {
    pub fn new(items: &'a StreamVec, width: Length, height: Length) -> ColumnLayout<'a> {
        let limit = items.0.len();
        let mut nodes = vec![None; limit+1];
        let mut nodes_col = vec![None; limit+1];
        nodes[0] = Some(LineBreak::default());
        nodes_col[0] = Some(ColumnBreak::default());

        let mut layout = ColumnLayout {
            para: ParagraphLayout {
                nodes,
                items: &items.0,
                width,
                last: 0
            },
            nodes_col,
            height,
        };
        layout.run();
        layout
    }
    pub fn columns<'l>(&'l self) -> Columns<'l, 'a> {
        Columns::new(self)
    }
    fn run(&mut self) {
        let mut last = 0;
        for start in 0 .. self.para.items.len() {
            match self.para.nodes[start] {
                Some(b) => {
                    last = self.para.complete_line(
                        start,
                        Context::new(start, b.score)
                    );
                    self.compute_column(start, false);
                },
                None => {}
            }
        }
        self.compute_column(last, true);

        if self.nodes_col[last].is_none() {
            for i in 0 .. last {
                println!("{:3} {:?}", i, self.para.items[i]);
                if let Some(b) = self.para.nodes[i] {
                    println!("     {:?}", b);
                }
                if let Some(l) = self.nodes_col[i] {
                    println!("     {:?}", l);
                }
            }
        }

        self.para.last = last;
    }

    fn num_lines_penalty(&self, n: usize) -> f32 {
        match n {
            1 => -20.0,
            2 => -2.0,
            _ => 0.0
        }
    }
    fn fill_penalty(&self, fill: Length) -> f32 {
        -10.0 * ((self.height - fill) / self.height)
    }

    fn compute_column(&mut self, n: usize, is_last: bool) -> bool {
        //                                        measure:
        let mut num_lines_before_end = 0;      // - lines before the break; reset between paragraphs
        let mut num_lines_at_last_break = 0;   // - lines after the previous break; count until the last paragraph starts
        let mut is_last_paragraph = true;
        let mut height = Length::zero();
        let mut last = n;
        let mut found = false;
        
        loop {
            let last_node = self.para.nodes[last].unwrap();
                        
            if last > 0 {
                match self.para.items[last-1] {
                    Entry::Linebreak(_) => {
                        is_last_paragraph = false;
                        num_lines_before_end = 0;
                    },
                    Entry::Space { .. } => {
                        num_lines_before_end += 1;

                        if is_last_paragraph {
                            num_lines_at_last_break += 1;
                        }
                    }
                    ref e => panic!("found: {:?}", e)
                }
                
                height += last_node.height;

                if height > self.height {
                    break;
                }
            }

            if let Some(column) = self.nodes_col[last] {
                let mut score = column.score
                    + self.num_lines_penalty(num_lines_at_last_break)
                    + self.num_lines_penalty(num_lines_before_end);
                
                if !is_last {
                    score += self.fill_penalty(height);
                }
            
                match self.nodes_col[n] {
                    Some(column) if column.score > score => {},
                    _ => {
                        self.nodes_col[n] = Some(ColumnBreak {
                            prev: last,
                            score: score
                        });
                        
                        found = true;
                    }
                }
            }

            if last == 0 {
                break;
            }
            last = last_node.prev;
        }
        
        found
    }
}

#[derive(Debug)]
pub struct Columns<'l, 'a: 'l> {
    layout:     &'l ColumnLayout<'a>,
    columns:    Vec<usize>
}
impl<'l, 'a: 'l> Columns<'l, 'a> {
    fn new(layout: &'l ColumnLayout<'a>) -> Self {
        let mut columns = Vec::new();
        let mut last = layout.para.last;
        while last > 0 {
            columns.push(last);
            last = layout.nodes_col[last].unwrap().prev;
        }
        Columns {
            layout: layout,
            columns: columns
        }
    }
}
impl<'l, 'a: 'l> Iterator for Columns<'l, 'a> {
    type Item = Column<'l, 'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.columns.pop().map(|last| Column::new(
            self.layout.nodes_col[last].unwrap().prev,
            last,
            &self.layout.para
        ))
    }
}

#[derive(Debug)]
pub struct Column<'l, 'a: 'l> {
    lines:      Vec<usize>, // points to the end of each line
    layout:     &'l ParagraphLayout<'a>,
    y:          Length
}
impl<'l, 'a: 'l> Column<'l, 'a> {
    fn new(first: usize, mut last: usize, layout: &'l ParagraphLayout<'a>) -> Self {
        let mut lines = Vec::new();
        while last > first {
            lines.push(last);
            last = layout.nodes[last].unwrap().prev;
        }
        
        Column {
            lines: lines,
            layout: layout,
            y: Length::zero()
        }
    }
}
impl<'l, 'a: 'l> Iterator for Column<'l, 'a> {
    type Item = (Length, Line<'l, 'a>);
    
    fn next(&mut self) -> Option<Self::Item> {
        self.lines.pop().map(|last| {
            let b = self.layout.nodes[last].unwrap();
            self.y += b.height;
            
            (self.y, Line {
                layout:   self.layout,
                pos:      b.prev,
                branches: 0,
                measure:  FlexMeasure::zero(),
                line:     b,
                end:      last-1
            })
        })
    }
}

#[derive(Debug)]
pub struct Line<'l, 'a: 'l> {
    layout:     &'l ParagraphLayout<'a>,
    pos:        usize,
    end:        usize,
    branches:   usize,
    measure:    FlexMeasure,
    line:       LineBreak
}
impl <'l, 'a: 'l> Line<'l, 'a> {
    pub fn height(&self) -> Length {
        self.measure.height
    }
}

impl<'l, 'a: 'l> Iterator for Line<'l, 'a> {
    type Item = (Length, Item, Tag);
    fn next(&mut self) -> Option<Self::Item> {
        while self.pos < self.end {
            let pos = self.pos;
            self.pos += 1;

            match self.layout.items[pos] {
                Entry::Word(w, m, font, tag) => {
                    let x = self.measure.at(self.line.factor);
                    self.measure += m;
                    return Some((x, Item::Word(w, font), tag));
                },
                Entry::Punctuation(s, m, font, tag) => {
                    let x = self.measure.at(self.line.factor);
                    self.measure += m;
                    return Some((x, Item::Symbol(s, font), tag));
                },
                Entry::Space(_, s) => {
                    self.measure += s;
                },
                Entry::Object(key, m, tag) => {
                    let x = self.measure.at(self.line.factor);
                    self.measure += m;
                    let width = m.at(self.line.factor);
                    return Some((x, Item::Object(key, width), tag));
                }
                Entry::BranchEntry(len) => {
                    if self.line.path & (1<<self.branches) == 0 {
                        // not taken
                        self.pos += len;
                    }
                    self.branches += 1;
                },
                Entry::BranchExit(skip) => self.pos += skip,
                Entry::Linebreak(_) => unreachable!(),
                _ => {}
            }
        }
        
        None
    }
}
