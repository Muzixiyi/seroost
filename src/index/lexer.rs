use std::{iter::Peekable, str::CharIndices};

#[derive(Debug)]
pub struct Lexer<'a> {
    content: &'a str,
    char_indices: Peekable<CharIndices<'a>>,
}

impl<'a> Lexer<'a> {
    pub fn new(content: &'a str) -> Self {
        Self {
            content,
            char_indices: content.char_indices().peekable(),
        }
    }

    fn trim_left(&mut self) {
        while let Some(&(_, c)) = self.char_indices.peek()
            && c.is_ascii_whitespace()
        {
            self.char_indices.next();
        }
    }

    fn chop_while(&mut self, predicate: impl Fn(char) -> bool) -> Option<&'a str> {
        let &(start_index, c) = self.char_indices.peek()?;
        if !predicate(c) {
            return None;
        }
        while let Some(&(_, c)) = self.char_indices.peek()
            && predicate(c)
        {
            self.char_indices.next();
        }

        let end_index = self
            .char_indices
            .peek()
            .map(|&(index, _)| index)
            .unwrap_or(self.content.len());

        Some(&self.content[start_index..end_index])
    }

    fn next_token(&mut self) -> Option<&'a str> {
        self.trim_left();

        let &(_, c) = self.char_indices.peek()?;

        if c.is_numeric() {
            return self.chop_while(|x| x.is_numeric());
        }

        if c.is_alphabetic() {
            return self.chop_while(|x| x.is_alphanumeric());
        }

        self.char_indices
            .next()
            .map(|(index, c)| &self.content[index..index + c.len_utf8()])
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_token()
    }
}
