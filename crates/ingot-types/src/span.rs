/// A byte-range span in source code. 8 bytes, Copy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Span {
    pub offset: u32,
    pub len: u32,
}

impl Span {
    pub fn new(offset: u32, len: u32) -> Self {
        Self { offset, len }
    }

    /// A zero-length span at offset 0, used for synthetic nodes.
    pub fn dummy() -> Self {
        Self { offset: 0, len: 0 }
    }

    /// Merge two spans into one covering both (and any gap between them).
    pub fn merge(self, other: Span) -> Span {
        let start = self.offset.min(other.offset);
        let end = (self.offset + self.len).max(other.offset + other.len);
        Span {
            offset: start,
            len: end - start,
        }
    }
}

impl From<Span> for miette::SourceSpan {
    fn from(s: Span) -> Self {
        miette::SourceSpan::new((s.offset as usize).into(), s.len as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn span_size() {
        assert_eq!(std::mem::size_of::<Span>(), 8);
    }

    #[test]
    fn merge_adjacent() {
        let a = Span::new(0, 3);
        let b = Span::new(3, 4);
        assert_eq!(a.merge(b), Span::new(0, 7));
    }

    #[test]
    fn merge_overlapping() {
        let a = Span::new(2, 5);
        let b = Span::new(4, 6);
        assert_eq!(a.merge(b), Span::new(2, 8));
    }

    #[test]
    fn merge_with_gap() {
        let a = Span::new(0, 2);
        let b = Span::new(10, 3);
        assert_eq!(a.merge(b), Span::new(0, 13));
    }

    #[test]
    fn dummy_is_zero() {
        let d = Span::dummy();
        assert_eq!(d.offset, 0);
        assert_eq!(d.len, 0);
    }

    #[test]
    fn into_miette_source_span() {
        let s = Span::new(5, 10);
        let ms: miette::SourceSpan = s.into();
        assert_eq!(ms.offset(), 5);
        assert_eq!(ms.len(), 10);
    }
}
