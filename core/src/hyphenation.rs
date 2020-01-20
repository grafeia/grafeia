use std::path::Path;
use std::io;
use std::ops;
use std::fs;
use std::collections::BTreeMap;
use fst::{Map, MapBuilder, Error};

#[derive(Clone)]
pub struct Hyphen {
    pos:    u8
}
impl Hyphen {
    pub fn at(pos: usize) -> Hyphen {
        assert!(pos < 256);
        Hyphen { pos: pos as u8 }
    }
    
    /// position specifies the char number not the byte count.
    pub fn apply<'a>(&self, word: &'a str) -> (&'a str, &'a str) {
        let p = self.pos as usize;
        let mut iter = word.chars();
        
        // count p chars 
        for _ in 0 .. p {
            iter.next();
        }
        //and everything left is the second part
        let second = iter.as_str();
        
        // the first part is everything but the last part
        let first = &word[0 .. word.len() - second.len()];
        (first, second)
    }
}

#[derive(Default, Clone)]
pub struct Hyphens {
    data:   u64
}
impl ops::Shl<Hyphen> for Hyphens {
    type Output = Hyphens;
    fn shl(self, rhs: Hyphen) -> Hyphens {
        assert!(self.data < 1<<56);
        Hyphens { data : self.data << 8 | rhs.pos as u64 }
    }
}
impl ops::ShlAssign<Hyphen> for Hyphens {
    fn shl_assign(&mut self, rhs: Hyphen) {
        assert!(self.data < 1<<56);
        self.data = self.data << 8 | rhs.pos as u64;
    }
}
impl Hyphens {
    pub fn iter(&self) -> HyphenIter {
        HyphenIter { data: self.data }
    }
    pub fn len(&self) -> usize {
        self.iter().count()
    }
}

pub struct HyphenIter {
    data:   u64
}
impl Iterator for HyphenIter {
    type Item = Hyphen;
    
    fn next(&mut self) -> Option<Hyphen> {
        if self.data > 0 {
            let h = Hyphen { pos: self.data as u8 };
            self.data >>= 8;
            Some(h)
        } else {
            None
        }
    }
}

pub struct Hyphenator {
    map:        Map,
    changes:    BTreeMap<String, Hyphens>
}

struct Entry {
    key:        String,
    value:      Hyphens
}

impl Hyphenator {
    pub fn add(&mut self, word: String, hyphens: Hyphens) {
        self.changes.insert(word, hyphens);
    }
    pub fn get(&self, word: &str) -> Option<Hyphens> {
        let lower = word.to_lowercase();
        match self.changes.get(&lower) {
            Some(h) => Some(h.clone()),
            None => match self.map.get(&lower) {
                Some(v) => Some(Hyphens { data: v }),
                None => None
            }
        }
    }
    
    pub fn load(data: Vec<u8>) -> Result<Hyphenator, Error> {
        match Map::from_bytes(data) {
            Ok(map) => Ok(Hyphenator {
                map: map,
                changes: BTreeMap::new()
            }),
            Err(e) => Err(e)
        }
    }
    
    pub fn empty() -> Hyphenator {
        use std::iter;
        Hyphenator {
            map: Map::from_iter(iter::empty::<(&str, u64)>()).unwrap(),
            changes: BTreeMap::new()
        }
    }
    
    // TODO: Asyncify
    pub fn save(&self, path: &Path) {
        let fst_tmp = path.with_extension("fst.part");
        
        if self.changes.len() == 0 {
            return; // nothing to do
        }
        
        let mut builder = MapBuilder::new(
            io::BufWriter::new(
                fs::File::create(&fst_tmp).unwrap()
            )
        ).unwrap();
        
        {
            let mut a_iter = self.map.stream();
            let mut b_iter = self.changes.iter();
            // items are sorted in increasing order
            // => insert smaller values first
            
            let mut next_a = || {
                use std::str;
                use fst::Streamer;
                a_iter.next().map(|(key, val)| { Entry {
                        key: str::from_utf8(key).unwrap().to_owned(),
                        value: Hyphens { data: val }
                } })
            };
            let mut next_b = || {
                b_iter.next()
                .map(|(key, value)| { Entry {
                    key: key.clone(),
                    value: value.clone()
                } } )
            };
            let mut insert = |e: &Entry| {
                builder.insert(e.key.as_str().as_bytes(), e.value.data).expect("out of order");
            };
            
            let mut o_a: Option<Entry> = next_a();
            let mut o_b: Option<Entry> = next_b();
            
            loop {
                use std::cmp::{Ordering};
                
                enum Advance {
                    A,
                    B,
                    AB
                };
                
                let advance = match (&o_a, &o_b) {
                    (&Some(ref a), &Some(ref b)) => match a.key.cmp(&b.key) {
                        Ordering::Less => {
                            insert(a);
                            Advance::A
                        },
                        Ordering::Equal => {
                            insert(b);
                            Advance::AB
                        },
                        Ordering::Greater => {
                            insert(b);
                            Advance::B
                        }
                    },
                    (&Some(ref a), &None) => {
                        insert(a);
                        Advance::A
                    },
                    (&None, &Some(ref b)) => {
                        insert(b);
                        Advance::B
                    },
                    (&None, &None) => break
                };
                
                match advance {
                    Advance::A => {
                        o_a = next_a();
                    },
                    Advance::B => {
                        o_b = next_b();
                    },
                    Advance::AB => {
                        o_a = next_a();
                        o_b = next_b();
                    }
                }
            }
        }
        builder.finish().unwrap();
        
        fs::rename(fst_tmp, path).expect("could not rename tempfile");
    }
    
    // TODO: Asyncify
    pub fn add_hyphenlist(&mut self, path: &Path) {
        use std::io::BufRead;
        let reader = io::BufReader::new(
            fs::File::open(path).unwrap()
        );
        
        for line in reader.lines() {
            let mut hyphens = Hyphens::default();
            let mut off = 0;
            let word: String = line.unwrap()
            .chars()
            .enumerate()
            .filter_map(|(pos, c)|
                if c == '|' {
                    hyphens <<= Hyphen::at(pos-off);
                    off += 1;
                    None
                } else {
                    Some(c)
                }
            ).collect();
            
            self.add(word, hyphens);
        }
    }
}