//! Aruã's minimal semantic document. Structural tags are validated by the parser;
//! only the title and ordered text blocks are retained. This is not a full DOM.

#[derive(Debug, PartialEq, Eq)]
pub struct Document {
    pub title: String,
    pub blocks: Vec<Block>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Block {
    Heading(String),
    Paragraph(String),
}

impl Block {
    pub fn text(&self) -> &str {
        match self {
            Self::Heading(text) | Self::Paragraph(text) => text,
        }
    }
}
