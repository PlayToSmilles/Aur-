//! Deliberately strict, non-HTML5 tokenizer and parser. No regex, implicit tags,
//! attribute processing, browser recovery rules or execution of document content.

use crate::document::{Block, Document};
use std::{fmt, fs::File, io::Read, path::Path};

pub const MAX_DOCUMENT_BYTES: usize = 1024 * 1024;

#[derive(Debug)]
pub struct HtmlError(pub String);

impl fmt::Display for HtmlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for HtmlError {}

pub fn load(path: &Path) -> Result<Document, HtmlError> {
    let file = File::open(path)
        .map_err(|e| HtmlError(format!("Não foi possível abrir {}: {e}", path.display())))?;
    let mut bytes = Vec::new();
    file.take((MAX_DOCUMENT_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|e| HtmlError(format!("Não foi possível ler {}: {e}", path.display())))?;
    parse_bytes(&bytes)
}

pub fn parse_bytes(bytes: &[u8]) -> Result<Document, HtmlError> {
    if bytes.len() > MAX_DOCUMENT_BYTES {
        return Err(HtmlError("Documento excede o limite de 1 MiB.".into()));
    }
    let text = std::str::from_utf8(bytes)
        .map_err(|e| HtmlError(format!("O documento não é UTF-8 válido: {e}")))?;
    parse(text)
}

fn html_space(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\r' | '\n' | '\u{000c}')
}

fn collapse(text: &str) -> String {
    text.split(html_space).filter(|s| !s.is_empty()).collect::<Vec<_>>().join(" ")
}

fn decode(text: &str) -> Result<String, HtmlError> {
    let mut out = String::new();
    let mut rest = text;
    while let Some(start) = rest.find('&') {
        out.push_str(&rest[..start]);
        rest = &rest[start + 1..];
        let end = rest.find(';').ok_or_else(|| HtmlError("Entidade sem ponto e vírgula.".into()))?;
        let entity = &rest[..end];
        let decoded = match entity {
            "amp" => '&', "lt" => '<', "gt" => '>', "quot" => '"',
            "apos" => '\'', "nbsp" => '\u{00a0}',
            _ => {
                let number = if let Some(hex) = entity.strip_prefix("#x").or_else(|| entity.strip_prefix("#X")) {
                    if !hex.is_empty() && hex.bytes().all(|b| b.is_ascii_hexdigit()) { u32::from_str_radix(hex, 16).ok() } else { None }
                } else if let Some(decimal) = entity.strip_prefix('#') {
                    if decimal.bytes().all(|b| b.is_ascii_digit()) { decimal.parse().ok() } else { None }
                } else { None };
                number.and_then(char::from_u32)
                    .filter(|c| !c.is_control() || html_space(*c))
                    .ok_or_else(|| HtmlError(format!("Entidade não suportada: &{entity};")))?
            }
        };
        out.push(decoded);
        rest = &rest[end + 1..];
    }
    out.push_str(rest);
    Ok(out)
}

#[derive(Debug)]
enum Token<'a> { Open(String), Close(String), Text(&'a str), Doctype, Comment }

struct Tokenizer<'a> { source: &'a str, offset: usize }

impl<'a> Tokenizer<'a> {
    fn next(&mut self) -> Result<Option<Token<'a>>, HtmlError> {
        let rest = &self.source[self.offset..];
        if rest.is_empty() { return Ok(None); }
        if !rest.starts_with('<') {
            let end = rest.find('<').unwrap_or(rest.len());
            self.offset += end;
            return Ok(Some(Token::Text(&rest[..end])));
        }
        if let Some(comment) = rest.strip_prefix("<!--") {
            let end = comment.find("-->").ok_or_else(|| HtmlError("Comentário não terminado.".into()))?;
            if comment[..end].contains("--") { return Err(HtmlError("Comentário com -- interno não suportado.".into())); }
            self.offset += 4 + end + 3;
            return Ok(Some(Token::Comment));
        }
        let end = rest.find('>').ok_or_else(|| HtmlError("Tag não terminada.".into()))?;
        let inside = &rest[1..end];
        self.offset += end + 1;
        if inside.eq_ignore_ascii_case("!doctype html") { return Ok(Some(Token::Doctype)); }
        let (closing, name) = match inside.strip_prefix('/') { Some(n) => (true, n), None => (false, inside) };
        let name = name.trim_end_matches(html_space);
        if name.is_empty() || !name.bytes().all(|b| b.is_ascii_alphanumeric()) {
            return Err(HtmlError(format!("Tag, atributo ou construção não suportada: <{inside}>")));
        }
        let name = name.to_ascii_lowercase();
        if !matches!(name.as_str(), "html" | "head" | "title" | "body" | "h1" | "p") {
            return Err(HtmlError(format!("Elemento não suportado: <{name}>")));
        }
        Ok(Some(if closing { Token::Close(name) } else { Token::Open(name) }))
    }
}

pub fn parse(source: &str) -> Result<Document, HtmlError> {
    if source.len() > MAX_DOCUMENT_BYTES { return Err(HtmlError("Documento excede o limite de 1 MiB.".into())); }
    let source = source.strip_prefix('\u{feff}').unwrap_or(source);
    if source.chars().any(|c| c.is_control() && !html_space(c)) {
        return Err(HtmlError("Caractere de controle não suportado.".into()));
    }
    let mut tokenizer = Tokenizer { source, offset: 0 };
    let mut stack: Vec<String> = Vec::new();
    let (mut seen_html, mut seen_head, mut seen_body, mut seen_title, mut seen_doctype) = (false, false, false, false, false);
    let mut document = Document { title: String::new(), blocks: Vec::new() };
    let mut text = String::new();
    while let Some(token) = tokenizer.next()? {
        match token {
            Token::Comment => {},
            Token::Doctype => {
                if seen_html || seen_doctype { return Err(HtmlError("DOCTYPE deve aparecer uma vez, antes de html.".into())); }
                seen_doctype = true;
            }
            Token::Open(name) => {
                let parent = stack.last().map(String::as_str);
                let allowed = match name.as_str() {
                    "html" if parent.is_none() && !seen_html => { seen_html = true; true },
                    "head" if parent == Some("html") && !seen_head && !seen_body => { seen_head = true; true },
                    "body" if parent == Some("html") && !seen_body => { seen_body = true; true },
                    "title" if parent == Some("head") && !seen_title => { seen_title = true; true },
                    "h1" | "p" if parent == Some("body") => true,
                    _ => false,
                };
                if !allowed { return Err(HtmlError(format!("Elemento <{name}> fora da estrutura suportada."))); }
                if matches!(name.as_str(), "title" | "h1" | "p") { text.clear(); }
                stack.push(name);
            }
            Token::Close(name) => {
                if stack.last() != Some(&name) { return Err(HtmlError(format!("Fechamento inesperado: </{name}>"))); }
                let content = if matches!(name.as_str(), "title" | "h1" | "p") { collapse(&text) } else { String::new() };
                match name.as_str() {
                    "title" => document.title = content,
                    "h1" => document.blocks.push(Block::Heading(content)),
                    "p" => document.blocks.push(Block::Paragraph(content)),
                    _ => {},
                }
                stack.pop();
            }
            Token::Text(value) => {
                if matches!(stack.last().map(String::as_str), Some("title" | "h1" | "p")) {
                    text.push_str(&decode(value)?);
                } else if !value.chars().all(html_space) {
                    return Err(HtmlError("Texto permitido apenas dentro de title, h1 ou p.".into()));
                }
            }
        }
    }
    if !seen_html || !seen_body || !stack.is_empty() { return Err(HtmlError("Documento incompleto: html/body e fechamentos explícitos são obrigatórios.".into())); }
    if document.title.is_empty() { document.title = "Aruã Mark I".into(); }
    Ok(document)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn page(body: &str) -> String { format!("<html><body>{body}</body></html>") }

    #[test]
    fn parses_the_requested_document_in_order() {
        let doc = parse(include_str!("../examples/mark_i.html")).unwrap();
        assert_eq!(doc.title, "Aruã Mark I");
        assert_eq!(doc.blocks, vec![
            Block::Heading("Olá, Brasil!".into()),
            Block::Paragraph("Este é o primeiro documento renderizado pelo Aruã Engine.".into()),
            Block::Paragraph("Construído do zero, uma Mark de cada vez.".into()),
        ]);
    }

    #[test]
    fn handles_bom_case_comments_and_html_whitespace() {
        let doc = parse("\u{feff}<!DOCTYPE html><!-- ok --><HTML><HEAD><TITLE>  T\n teste </TITLE></HEAD><BODY><P>Olá\n  Brasil<!--a-->!</P></BODY></HTML>").unwrap();
        assert_eq!(doc.title, "T teste");
        assert_eq!(doc.blocks, vec![Block::Paragraph("Olá Brasil!".into())]);
    }

    #[test]
    fn decodes_supported_entities_without_creating_markup() {
        let doc = parse(&page("<p>&lt;p&gt; &amp; &quot;x&quot; &apos;y&apos; &#227; &#x1F1E7; a&nbsp;b</p>")).unwrap();
        assert_eq!(doc.blocks[0].text(), "<p> & \"x\" 'y' ã 🇧 a\u{00a0}b");
    }

    #[test]
    fn rejects_unknown_elements_attributes_and_execution_content() {
        for body in ["<div>x</div>", "<p class='x'>x</p>", "<script>alert(1)</script>", "<p/>", "<p><h1>x</h1></p>", "texto"] {
            assert!(parse(&page(body)).is_err(), "accepted: {body}");
        }
    }

    #[test]
    fn rejects_malformed_structure() {
        for input in ["", "<html></html>", "<html><body><p>x</body></html>", "<html><body></body>", "<html><body></body><head></head></html>", "<html><body></body><body></body></html>", "<html><head><title>a</title><title>b</title></head><body></body></html>", "<html><body></body></html><html><body></body></html>", "<!doctype html><!doctype html><html><body></body></html>", "<html><body></body></html>extra"] {
            assert!(parse(input).is_err(), "accepted: {input}");
        }
    }

    #[test]
    fn rejects_unterminated_tokens_and_bad_entities() {
        for body in ["<!--oops", "<p", "<p>&missing;</p>", "<p>&amp</p>", "<p>&#0;</p>", "<p>&#x+41;</p>", "<p>&#xD800;</p>", "<p>&#1114112;</p>", "<p>\0</p>"] {
            assert!(parse(&page(body)).is_err(), "accepted: {body}");
        }
    }

    #[test]
    fn empty_blocks_and_optional_head_have_defined_results() {
        let doc = parse(&page("<h1></h1><p> \n </p>")).unwrap();
        assert_eq!(doc.title, "Aruã Mark I");
        assert_eq!(doc.blocks, vec![Block::Heading(String::new()), Block::Paragraph(String::new())]);
    }

    #[test]
    fn rejects_invalid_utf8_and_oversized_input() {
        assert!(parse_bytes(&[0xff, 0xfe]).unwrap_err().0.contains("UTF-8"));
        assert!(parse_bytes(&vec![b' '; MAX_DOCUMENT_BYTES + 1]).is_err());
    }

    #[test]
    fn loads_utf8_from_disk_and_reports_missing_file() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        assert_eq!(load(&root.join("examples/mark_i.html")).unwrap().blocks.len(), 3);
        assert!(load(&root.join("target/nonexistent-mark-i-file.html")).unwrap_err().0.contains("abrir"));
    }
}
