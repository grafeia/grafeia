use font::GlyphId;
use crate::content::FontFace;

pub fn grapheme_indices(face: &FontFace, text: &str) -> Vec<usize> {
    if let Some(gsub) = face.get_gsub() {
        let mut char_indices = text.char_indices();
        let mut grapheme_indices = Vec::with_capacity(text.len());
        let gids: Vec<GlyphId> = text.chars().map(|c| face.gid_for_unicode_codepoint(c as u32).unwrap_or(face.get_notdef_gid())).collect();
        let mut pos = 0;
        let mut chars = text.chars();
    'a: while let Some(&first) = gids.get(pos) {
            grapheme_indices.push(char_indices.next().unwrap().0);
            pos += 1;
            if let Some(subs) = gsub.substitutions(first) {
                for (sub, glyph) in subs {
                    if let Some(len) = sub.matches(&gids[pos ..]) {
                        pos += len;
                        char_indices.by_ref().take(len).count();
                        continue 'a;
                    }
                }
            }
        }
        grapheme_indices
    } else {
        text.char_indices().map(|(idx, _)| idx).collect()
    }
}

pub fn build_gids(face: &FontFace, text: &str) -> Vec<GlyphId> {
    use std::iter::once;
    let mut gids: Vec<GlyphId> = text.chars().map(|c| face.gid_for_unicode_codepoint(c as u32).unwrap_or(face.get_notdef_gid())).collect();
    if let Some(gsub) = face.get_gsub() {
        let mut pos = 0;
    'a: while let Some(&first) = gids.get(pos) {
            pos += 1;
            if let Some(subs) = gsub.substitutions(first) {
                for (sub, glyph) in subs {
                    if let Some(len) = sub.matches(&gids[pos ..]) {
                        gids.splice(pos-1 .. pos+len, once(glyph));
                        continue 'a;
                    }
                }
            }
        }
    }
    gids
}
