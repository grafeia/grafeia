use grafeia_core::*;
use std::io;
use std::fmt::Write;
use std::collections::HashMap;
use pathfinder_content::{outline::Outline, segment::SegmentKind};
use pathfinder_geometry::{
    rect::RectF,
    vector::Vector2F,
    transform2d::Transform2F,
};
use grafeia_core::layout::Column;
use grafeia_core::draw::{
    RenderItem,
    Cache
};


use std::fmt;
struct PdfRect(RectF);
impl fmt::Display for PdfRect {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let ll = self.0.lower_left();
        let ur = self.0.upper_right();
        write!(f, "[{} {} {} {}]", ll.x(), ll.y(), ur.x(), ur.y())
    }
}

struct PdfTransform(Transform2F);
impl fmt::Display for PdfTransform {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let m = self.0.matrix;
        let v = self.0.vector;
        write!(f, "{} {} {} {} {} {}", m.m11(), m.m12(), m.m21(), m.m22(), v.x(), v.y())
    }
}

struct Counter<T> {
    inner: T,
    count: u64
}
impl<T> Counter<T> {
    pub fn new(inner: T) -> Counter<T> {
        Counter {
            inner,
            count: 0
        }
    }
    pub fn pos(&self) -> u64 {
        self.count
    }
}
impl<W: io::Write> io::Write for Counter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self.inner.write(buf) {
            Ok(n) => {
                self.count += n as u64;
                Ok(n)
            },
            Err(e) => Err(e)
        }
    }
    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }

    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        self.inner.write_all(buf)?;
        self.count += buf.len() as u64;
        Ok(())
    }
}

/// Represents a PDF internal object
struct PdfObject {
    contents: Vec<u8>,
    is_page: bool,
    is_xobject: bool,
    offset: Option<u64>,
}


struct Stream(String);
impl Stream {
    pub fn new() -> Stream {
        Stream(String::new())
    }
    pub fn move_to(&mut self, p: Vector2F) {
        writeln!(self.0, "{} {} m", p.x(), p.y()).unwrap();
    }

    pub fn line_to(&mut self, p: Vector2F) {
        writeln!(self.0, "{} {} l", p.x(), p.y()).unwrap();
    }

    pub fn cubic_to(&mut self, c1: Vector2F, c2: Vector2F, p: Vector2F) {
        writeln!(self.0, "{} {} {} {} {} {} c", c1.x(), c1.y(), c2.x(), c2.y(), p.x(), p.y()).unwrap();
    }
    pub fn fill(&mut self) {
        writeln!(self.0, "f").unwrap();
    }

    pub fn close(&mut self) {
        writeln!(self.0, "h").unwrap();
    }

    pub fn use_define(&mut self, n: usize, tr: Transform2F) {
        writeln!(self.0, "q {} cm /x{} Do Q", PdfTransform(tr), n).unwrap();
    }

    pub fn draw_path(&mut self, outline: Outline) {
        for contour in outline.contours() {
            for (segment_index, segment) in contour.iter().enumerate() {
                if segment_index == 0 {
                    self.move_to(segment.baseline.from());
                }

                match segment.kind {
                    SegmentKind::None => {}
                    SegmentKind::Line => self.line_to(segment.baseline.to()),
                    SegmentKind::Quadratic => {
                        let current = segment.baseline.from();
                        let c = segment.ctrl.from();
                        let p = segment.baseline.to();
                        let c1 = Vector2F::splat(2./3.) * c + Vector2F::splat(1./3.) * current;
                        let c2 = Vector2F::splat(2./3.) * c + Vector2F::splat(1./3.) * p;
                        self.cubic_to(c1, c2, p);
                    }
                    SegmentKind::Cubic => self.cubic_to(
                        segment.ctrl.from(),
                        segment.ctrl.to(),
                        segment.baseline.to()
                    )
                }
            }

            if contour.is_closed() {
                self.close();
            }
        }
    }
}
/// The top-level struct that represents a (partially) in-memory PDF file
pub struct Pdf {
    objects: Vec<PdfObject>,
    defines: HashMap<RenderItem, usize>
}
impl Pdf {
    /// Create a new blank PDF document
    #[inline]
    pub fn new() -> Self {
        Self {
            objects: vec![
                PdfObject {
                    contents: Vec::new(),
                    is_page: false,
                    is_xobject: false,
                    offset: None,
                },
                PdfObject {
                    contents: Vec::new(),
                    is_page: false,
                    is_xobject: false,
                    offset: None,
                },
            ],
            defines: HashMap::new()
        }
    }
    fn defined(&mut self, key: RenderItem, f: impl FnOnce() -> Outline) -> usize {
        if let Some(&n) = self.defines.get(&key) {
            return n;
        }
        let outline = f();
        let bbox = outline.bounds();
        let mut stream = Stream::new();
        stream.draw_path(outline);
        stream.fill();

        let mut dict = String::new();
        writeln!(dict, "<< /Type XObject");
        writeln!(dict, "/Subtype /Form");
        writeln!(dict, "/BBox {}", PdfRect(bbox));
        writeln!(dict, "/Length {} >>\nstream\n{}endstream\n", stream.0.len(), stream.0);

        let n = self.add_object(dict.into_bytes(), false, true);
        self.defines.insert(key, n);
        n
    }
    pub fn render_page(&mut self, cache: &Cache, storage: &Storage, target: &Target, design: &Design, column: Column) {
        let content_box: RectF = target.content_box.into();
        let mut stream = Stream::new();
        writeln!(stream.0, "{} cm", PdfTransform(Transform2F::row_major(1.0, 0.0, 0.0, -1.0, 0.0, content_box.height())));

        for (y, line) in column {
            for (x, size, item, tag) in line {
                let size: Vector2F = size.into();
                let p = content_box.origin() + Vector2F::new(x.value as f32, y.value as f32);
                let rect = RectF::new(p - Vector2F::new(0.0, size.y()), size);
                match item {
                    RenderItem::Word(key, part, font) => {
                        let n = self.defined(item, || {
                            let layout = cache.word_layout_cache.get(&(font, key, part)).unwrap();
                            let font = storage.get_font_face(font.font_face);
                            layout.render(font, Transform2F::default())
                        });
                        stream.use_define(n, Transform2F::from_translation(p));
                    }
                    RenderItem::Symbol(key, font) => {
                        let n = self.defined(item, || {
                            let layout = cache.symbol_layout_cache.get(&(font, key)).unwrap();
                            let font = storage.get_font_face(font.font_face);
                            layout.render(font, Transform2F::default())
                        });
                        stream.use_define(n, Transform2F::from_translation(p));
                    }
                    RenderItem::Object(key) => {
                        /*
                        let typ_design = design.get_type_or_default(storage.get_weave(tag.seq()).typ());
                        page.get_object(key).draw(typ_design, p, size.into(), &mut scene);
                        */
                    }
                    RenderItem::Empty => {}
                };
            }
        }
        
        let page_stream = format!("<< /Length {} >>\nstream\n{}endstream\n", stream.0.len(), stream.0).into_bytes();

        // Create the stream object for this page
        let stream_object_id = self.add_object(page_stream, false, false);

        // Create the page object, which describes settings for the whole page
        let page_object = format!("\
<< /Type /Page
/Parent 2 0 R
/MediaBox {}
/TrimBox {}
/Contents {} 0 R
>>\n",
        PdfRect(target.media_box.into()),
                PdfRect(target.trim_box.into()),
                stream_object_id
        ).into_bytes();
        self.add_object(page_object, true, false);
    }

    fn add_object(&mut self, data: Vec<u8>, is_page: bool, is_xobject: bool) -> usize {
        self.objects.push(PdfObject {
            contents: data,
            is_page,
            is_xobject,
            offset: None,
        });
        self.objects.len()
    }

    /// Write the in-memory PDF representation to disk
    pub fn write_to(&mut self, writer: impl io::Write) -> io::Result<()> {
        let mut out = Counter::new(writer);
        use std::io::Write;
        out.write_all(b"%PDF-1.7\n%\xB5\xED\xAE\xFB\n")?;

        let mut xobjects = String::from("<<\n");
        for n in self.defines.values() {
            writeln!(xobjects, "/x{} {} 0 R", n, n);
        }
        writeln!(xobjects, ">>");
        let xobjects_nr = self.add_object(xobjects.into_bytes(), false, false);

        // Write out each object
        for (idx, obj) in self.objects.iter_mut().enumerate().skip(2) {
            obj.offset = Some(out.pos());
            write!(out, "{} 0 obj\n", idx+1)?;
            out.write_all(&obj.contents)?;
            out.write_all(b"endobj\n")?;
        }

        // Write out the page tree object
        self.objects[1].offset = Some(out.pos());
        out.write_all(b"2 0 obj\n")?;
        out.write_all(b"<< /Type /Pages\n")?;
        writeln!(out, "/Resources {} 0 R", xobjects_nr);
        write!(out,
            "/Count {}\n",
            self.objects.iter().filter(|o| o.is_page).count()
        )?;
        out.write_all(b"/Kids [")?;
        for (idx, _obj) in self.objects.iter().enumerate().filter(|&(_, obj)| obj.is_page) {
            write!(out, "{} 0 R ", idx + 1)?;
        }
        out.write_all(b"] >>\nendobj\n")?;

        // Write out the catalog dictionary object
        self.objects[0].offset = Some(out.pos());
        out.write_all(b"1 0 obj\n<< /Type /Catalog\n/Pages 2 0 R >>\nendobj\n")?;

        // Write the cross-reference table
        let startxref = out.pos() + 1; // NOTE: apparently there's some 1-based indexing??
        out.write_all(b"xref\n")?;
        write!(out, "0 {}\n", self.objects.len() + 1)?;
        out.write_all(b"0000000000 65535 f \n")?;

        for obj in &self.objects {
            write!(out, "{:010} 00000 f \n", obj.offset.unwrap())?;
        }

        // Write the document trailer
        out.write_all(b"trailer\n")?;
        write!(out, "<< /Size {}\n", self.objects.len())?;
        out.write_all(b"/Root 1 0 R >>\n")?;

        // Write the offset to the xref table
        write!(out, "startxref\n{}\n", startxref)?;

        // Write the PDF EOF
        out.write_all(b"%%EOF")?;

        Ok(())
    }
}


fn main() {
    use std::fs::File;
    env_logger::init();

    let mut args = std::env::args().skip(1);
    let input = File::open(args.next().expect("no input given")).expect("can't open input file");
    let output = File::create(args.next().expect("no output file given")).expect("can't create output file");
    
    let state = State::load(input).unwrap();
    let mut cache = Cache::new();

    let layout = cache.layout(&state.storage, &state.design, &state.target, state.root);

    let mut pdf = Pdf::new();
    for column in layout.columns() {
        pdf.render_page(&cache, &state.storage, &state.target, &state.design, column);
    }
    pdf.write_to(output);
}
