use std::borrow::Cow;

pub trait TermProcessor {
    fn process<'a>(&self, s: &'a str) -> Cow<'a, str>;
}

pub enum Processor {
    Raw(Raw),
    Lowercase(Lowercase),
    Uppercase(Uppercase),
    Stemming(Stemming),
}
impl TermProcessor for Processor {
    fn process<'a>(&self, s: &'a str) -> Cow<'a, str> {
        match self {
            Processor::Raw(raw_processor) => raw_processor.process(s),
            Processor::Lowercase(lowercase_processor) => lowercase_processor.process(s),
            Processor::Uppercase(uppercase_processor) => uppercase_processor.process(s),
            Processor::Stemming(stemming_processor) => stemming_processor.process(s),
        }
    }
}

#[derive(Default)]
pub struct Raw;
impl TermProcessor for Raw {
    fn process<'a>(&self, s: &'a str) -> Cow<'a, str> {
        Cow::Borrowed(s)
    }
}

#[derive(Default)]
pub struct Lowercase;
impl TermProcessor for Lowercase {
    fn process<'a>(&self, s: &'a str) -> Cow<'a, str> {
        if s.chars().any(|c| c.is_uppercase()) {
            Cow::Owned(s.to_lowercase())
        } else {
            Cow::Borrowed(s)
        }
    }
}

#[derive(Default)]
pub struct Uppercase;
impl TermProcessor for Uppercase {
    fn process<'a>(&self, s: &'a str) -> Cow<'a, str> {
        if s.chars().any(|c| c.is_lowercase()) {
            Cow::Owned(s.to_uppercase())
        } else {
            Cow::Borrowed(s)
        }
    }
}

pub struct Stemming {
    pub stemmer: waken_snowball::Stemmer,
}
impl Stemming {
    pub fn new(stemmer: waken_snowball::Stemmer) -> Self {
        Self { stemmer }
    }
}

impl TermProcessor for Stemming {
    fn process<'a>(&self, s: &'a str) -> Cow<'a, str> {
        if s.chars().any(|c| c.is_uppercase()) {
            let lc = s.to_lowercase();
            let stemmed = self.stemmer.stem(&lc);
            Cow::Owned(stemmed.into_owned())
        } else {
            self.stemmer.stem(s)
        }
    }
}
