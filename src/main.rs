use latex::LatexWrite;
use std::{
    io::Write,
    io::{stdin, stdout, Read},
    ops::Deref,
};

use cssparser::{Delimiter, ParseError, Parser, ParserInput, Token};
use ego_tree::NodeRef;
use scraper::{Html, Node};

mod latex;

fn main() {
    let mut html = String::new();
    stdin()
        .read_to_string(&mut html)
        .expect("stdin to be a valid utf-8 string");

    // let html = include_str!("sample6.html");

    let doc = Html::parse_fragment(&html);
    let mut out = LatexWrite::from(stdout());

    for n in doc.root_element().children() {
        write_node(&mut out, n).expect("write node");
    }
}

fn write_node(out: &mut impl Write, node: NodeRef<Node>) -> Result<(), std::io::Error> {
    match node.value() {
        Node::Text(t) => write!(out, "{}", t.deref()),
        Node::Element(el) => {
            if el.name() == "p" {
                write!(out, "\n\n")?;
            }
            if let Some(sty) = el.attr("style") {
                if let Some(sty) = parse_style(sty) {
                    sty.write_start(out)?;
                    for n in node.children() {
                        write_node(out, n)?;
                    }
                    sty.write_end(out)?;
                } else {
                    for n in node.children() {
                        write_node(out, n)?;
                    }
                }
            } else {
                for n in node.children() {
                    write_node(out, n)?;
                }
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

// fn print_el(el: &Element) {
//     println!("name: {}", el.name());

//     if let Some(sty) = el.attr("style") {
//         if let Some(sty) = parse_style(sty) {
//             println!(
//                 "bold: {}, italic: {}, super: {}",
//                 sty.bold, sty.italic, sty.superscript
//             );
//         }
//     }
// }

enum RelevantProp {
    VerticalAlign,
    FontStyle,
    FontWeight,
    MarginLeft,
}

#[derive(Clone, Copy)]
struct Style {
    superscript: bool,
    italic: bool,
    bold: bool,
    quotation: bool,
}

impl Style {
    fn write_start(self, out: &mut impl Write) -> Result<(), std::io::Error> {
        if self.quotation {
            writeln!(out, "\\begin{{quote}}")
        } else {
            if self.bold {
                write!(out, "\\textbf{{")?;
            }
            if self.italic {
                write!(out, "\\textit{{")?;
            }
            if self.superscript {
                write!(out, "\\footnote{{")?;
            }
            Ok(())
        }
    }

    fn write_end(self, out: &mut impl Write) -> Result<(), std::io::Error> {
        if self.quotation {
            writeln!(out, "\n\\end{{quote}}")
        } else {
            if self.bold {
                write!(out, "}}")?;
            }
            if self.italic {
                write!(out, "}}")?;
            }
            if self.superscript {
                write!(out, "}}")?;
            }
            Ok(())
        }
    }
}

fn parse_style(style: &str) -> Option<Style> {
    let mut input = ParserInput::new(style);
    let mut input = Parser::new(&mut input);

    let mut style = Style {
        superscript: false,
        italic: false,
        bold: false,
        quotation: false,
    };

    loop {
        let token = match input.next() {
            Ok(t) => t,
            _ => break,
        };

        let prop = match *token {
            Token::Ident(ref attr) => match attr.deref() {
                "font" => {
                    let token = match input.next() {
                        Ok(t) => t,
                        _ => break,
                    };

                    match *token {
                        Token::Colon => {}
                        _ => {
                            continue;
                        }
                    };

                    style = input
                        .parse_until_after(Delimiter::Semicolon, parse_font)
                        .unwrap_or(style);
                    continue;
                }
                "font-weight" => RelevantProp::FontWeight,
                "font-style" => RelevantProp::FontStyle,
                "vertical-align" => RelevantProp::VerticalAlign,
                "margin-left" => RelevantProp::MarginLeft,
                _ => {
                    let token = match input.next() {
                        Ok(t) => t,
                        _ => break,
                    };

                    match *token {
                        Token::Colon => {}
                        _ => {
                            continue;
                        }
                    };

                    input
                        .parse_until_after(Delimiter::Semicolon, consume)
                        .unwrap_or(());

                    continue;
                }
            },
            _ => continue,
        };

        let token = match input.next() {
            Ok(t) => t,
            _ => break,
        };

        match *token {
            Token::Colon => {}
            _ => {
                continue;
            }
        };

        match prop {
            RelevantProp::VerticalAlign => {
                style.superscript = input
                    .parse_until_after(Delimiter::Semicolon, is_superscript)
                    .unwrap_or(false)
            }
            RelevantProp::FontStyle => {
                style.italic = input
                    .parse_until_after(Delimiter::Semicolon, is_italic)
                    .unwrap_or(false)
            }
            RelevantProp::FontWeight => {
                style.bold = input
                    .parse_until_after(Delimiter::Semicolon, is_bold)
                    .unwrap_or(false)
            }
            RelevantProp::MarginLeft => {
                style.quotation = input
                    .parse_until_after(Delimiter::Semicolon, is_quotation)
                    .unwrap_or(false)
            }
        }
    }

    Some(style)
}

fn is_superscript<'t>(input: &mut Parser<'t, '_>) -> Result<bool, ParseError<'t, ()>> {
    let token = input.next()?;

    match *token {
        Token::Ident(ref name) if name.deref() == "super" => Ok(true),
        _ => Ok(false),
    }
}

fn is_italic<'t>(input: &mut Parser<'t, '_>) -> Result<bool, ParseError<'t, ()>> {
    let token = input.next()?;

    match *token {
        Token::Ident(ref name) if name.deref() == "italic" => Ok(true),
        _ => Ok(false),
    }
}

fn is_bold<'t>(input: &mut Parser<'t, '_>) -> Result<bool, ParseError<'t, ()>> {
    let token = input.next()?;

    match *token {
        Token::Ident(ref name) => match name.deref() {
            "bold" | "bolder" => Ok(true),
            _ => Ok(false),
        },
        Token::Number {
            has_sign: _,
            value,
            int_value: _,
        } if value >= 700.0 => Ok(true),
        _ => Ok(false),
    }
}

fn is_quotation<'t>(input: &mut Parser<'t, '_>) -> Result<bool, ParseError<'t, ()>> {
    let token = input.next()?;
    match *token {
        Token::Dimension {
            has_sign: _,
            value,
            int_value: _,
            unit: _,
        } if value >= 50.0 => Ok(true),
        _ => Ok(false),
    }
}

fn parse_font<'t>(input: &mut Parser<'t, '_>) -> Result<Style, ParseError<'t, ()>> {
    let mut style = Style {
        superscript: false,
        italic: false,
        bold: false,
        quotation: false,
    };

    loop {
        let token = match input.next() {
            Ok(t) => t,
            _ => break,
        };

        match *token {
            Token::Ident(ref name) => match name.deref() {
                "bold" | "bolder" => style.bold = true,
                "italic" => style.italic = true,
                _ => {}
            },
            Token::Number {
                has_sign: _,
                value,
                int_value: _,
            } if value >= 700.0 => style.bold = true,
            _ => {}
        }
    }

    Ok(style)
}

fn consume<'t>(input: &mut Parser<'t, '_>) -> Result<(), ParseError<'t, ()>> {
    loop {
        let token = match input.next() {
            Ok(t) => t,
            _ => return Ok(()),
        };

        match *token {
            _ => {}
        }
    }
}
