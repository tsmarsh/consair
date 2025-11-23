use once_cell::sync::Lazy;
use std::fmt;
use std::sync::RwLock;
use string_interner::{DefaultBackend, DefaultSymbol, StringInterner};

static INTERNER: Lazy<RwLock<StringInterner<DefaultBackend>>> =
    Lazy::new(|| RwLock::new(StringInterner::default()));

/// A symbol that has been interned in the global string interner
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct InternedSymbol(DefaultSymbol);

impl InternedSymbol {
    /// Intern a string and return an InternedSymbol
    pub fn new(s: &str) -> Self {
        let mut interner = INTERNER.write().unwrap();
        InternedSymbol(interner.get_or_intern(s))
    }

    /// Resolve the interned symbol back to its string representation
    pub fn resolve(&self) -> String {
        let interner = INTERNER.read().unwrap();
        interner
            .resolve(self.0)
            .expect("Symbol should always be valid")
            .to_string()
    }

    /// Resolve the symbol and run a function with the string slice
    /// This is more efficient than resolve() which allocates a String
    pub fn with_str<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&str) -> R,
    {
        let interner = INTERNER.read().unwrap();
        let s = interner
            .resolve(self.0)
            .expect("Symbol should always be valid");
        f(s)
    }
}

impl fmt::Display for InternedSymbol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.with_str(|s| write!(f, "{s}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intern_same_string_returns_same_symbol() {
        let sym1 = InternedSymbol::new("foo");
        let sym2 = InternedSymbol::new("foo");
        assert_eq!(sym1, sym2);
    }

    #[test]
    fn test_intern_different_strings_returns_different_symbols() {
        let sym1 = InternedSymbol::new("foo");
        let sym2 = InternedSymbol::new("bar");
        assert_ne!(sym1, sym2);
    }

    #[test]
    fn test_resolve_returns_original_string() {
        let sym = InternedSymbol::new("hello");
        assert_eq!(sym.resolve(), "hello");
    }

    #[test]
    fn test_with_str() {
        let sym = InternedSymbol::new("test");
        let len = sym.with_str(|s| s.len());
        assert_eq!(len, 4);
    }

    #[test]
    fn test_display() {
        let sym = InternedSymbol::new("display-test");
        assert_eq!(format!("{sym}"), "display-test");
    }
}
