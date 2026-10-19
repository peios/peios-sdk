// SDDL conditional-expression text ↔ Condition AST.
//
// SDDL writes conditional ACE expressions in an infix syntax with
// roughly C-like operator precedence (MS-DTYP §2.5.1.2). This module
// handles three directions:
//
//   parse(s)        — text → Condition (recursive descent)
//   format(c)       — Condition → canonical text
//   decode_artx(b)  — `artx` bytecode (the binary form, what
//                     `Condition::encode` produces) → Condition
//
// `decode_artx` runs a postfix evaluator over the token stream, since
// the bytecode is already in reverse-Polish order. It's the inverse of
// `Condition::encode`.

use alloc::boxed::Box;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::security::sddl::condition::{CompareOp, Condition, MemberOp, Operand};

/// Errors produced by the conditional-expression text parser / artx
/// decoder.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum CondError {
    Eof,
    TooDeep,
    Unexpected(char, usize),
    UnknownToken(usize, String),
    BadLiteral(usize, String),
    Unbalanced(&'static str),
    Artx(&'static str),
}

pub type Result<T> = core::result::Result<T, CondError>;

impl core::fmt::Display for CondError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            CondError::TooDeep => write!(f, "condition exceeds the 128-level depth limit"),
            CondError::Eof => write!(f, "unexpected end of input"),
            CondError::Unexpected(c, off) => {
                write!(f, "unexpected character {c:?} at offset {off}")
            }
            CondError::UnknownToken(off, s) => {
                write!(
                    f,
                    "unknown operator or token starting at offset {off}: {s:?}"
                )
            }
            CondError::BadLiteral(off, s) => write!(f, "invalid literal at offset {off}: {s}"),
            CondError::Unbalanced(s) => write!(f, "unbalanced delimiter: {s}"),
            CondError::Artx(s) => write!(f, "artx: {s}"),
        }
    }
}

impl core::error::Error for CondError {}

// ---------------------------------------------------------------------------
// Public entry points
// ---------------------------------------------------------------------------

/// Parse an SDDL conditional expression. The input may be wrapped in a
/// single outer pair of parentheses (as it appears inside an ACE) or
/// not — both are accepted.
pub fn parse(s: &str) -> Result<Condition> {
    let s = s.trim();
    let mut p = Parser::new(s);
    let c = p.parse_expr()?;
    p.skip_ws();
    if !p.eof() {
        return Err(CondError::Unexpected(p.peek_char(), p.pos));
    }
    Ok(c.value)
}

/// Format a [`Condition`] as canonical SDDL conditional-expression text
/// (no outer parens).
pub fn format(c: &Condition) -> String {
    let mut out = String::new();
    fmt_condition(c, &mut out, 0);
    out
}

/// Decode the `artx`-prefixed bytecode form back to a [`Condition`].
/// Inverse of [`Condition::encode`].
pub fn decode_artx(bytes: &[u8]) -> Result<Condition> {
    if bytes.len() < 4 || &bytes[..4] != b"artx" {
        return Err(CondError::Artx("missing artx magic"));
    }
    let mut p = ArtxDecoder::new(&bytes[4..]);
    p.decode()
}

// ---------------------------------------------------------------------------
// Recursive-descent text parser
// ---------------------------------------------------------------------------

/// Bound both parser recursion and the resulting tree, including operands.
/// Scalar operands have depth zero; each condition or composite adds one.
const MAX_CONDITION_DEPTH: usize = 128;

#[derive(Debug)]
struct Bounded<T> {
    value: T,
    depth: usize,
}

impl<T> Bounded<T> {
    fn scalar(value: T) -> Self {
        Self { value, depth: 0 }
    }

    fn parent(child_depth: usize, build: impl FnOnce() -> T) -> Result<Self> {
        let depth = child_depth + 1;
        if depth > MAX_CONDITION_DEPTH {
            return Err(CondError::TooDeep);
        }
        Ok(Self {
            value: build(),
            depth,
        })
    }
}

struct Parser<'a> {
    src: &'a str,
    pos: usize,
    nesting: usize,
}

impl<'a> Parser<'a> {
    fn new(src: &'a str) -> Self {
        Self {
            src,
            pos: 0,
            nesting: 0,
        }
    }

    fn nested<T>(&mut self, parse: impl FnOnce(&mut Self) -> Result<T>) -> Result<T> {
        if self.nesting == MAX_CONDITION_DEPTH {
            return Err(CondError::TooDeep);
        }
        self.nesting += 1;
        let result = parse(self);
        self.nesting -= 1;
        result
    }

    fn eof(&self) -> bool {
        self.pos >= self.src.len()
    }

    fn rest(&self) -> &'a str {
        &self.src[self.pos..]
    }

    fn skip_ws(&mut self) {
        while !self.eof() {
            let b = self.src.as_bytes()[self.pos];
            if b == b' ' || b == b'\t' || b == b'\n' || b == b'\r' {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    fn peek_char(&self) -> char {
        self.rest().chars().next().unwrap_or('\0')
    }

    fn starts_with(&self, s: &str) -> bool {
        self.rest().as_bytes().starts_with(s.as_bytes())
    }

    fn starts_with_ci(&self, s: &str) -> bool {
        let r = self.rest().as_bytes();
        if r.len() < s.len() {
            return false;
        }
        for (i, b) in s.bytes().enumerate() {
            if !r[i].eq_ignore_ascii_case(&b) {
                return false;
            }
        }
        true
    }

    /// Match a keyword followed by a non-identifier character (or EOF).
    /// Case-insensitive.
    fn try_keyword(&mut self, kw: &str) -> bool {
        if !self.starts_with_ci(kw) {
            return false;
        }
        // Boundary check — keywords can't run into an identifier char.
        let next = self.src.as_bytes().get(self.pos + kw.len()).copied();
        if let Some(b) = next {
            if is_ident_continue(b) {
                return false;
            }
        }
        self.pos += kw.len();
        true
    }

    fn eat(&mut self, s: &str) -> bool {
        if self.starts_with(s) {
            self.pos += s.len();
            true
        } else {
            false
        }
    }

    /// Top-level: || has lowest precedence.
    fn parse_expr(&mut self) -> Result<Bounded<Condition>> {
        let mut lhs = self.parse_and()?;
        loop {
            self.skip_ws();
            if self.eat("||") {
                self.skip_ws();
                let rhs = self.parse_and()?;
                lhs = Bounded::parent(lhs.depth.max(rhs.depth), || lhs.value.or(rhs.value))?;
            } else {
                break;
            }
        }
        Ok(lhs)
    }

    fn parse_and(&mut self) -> Result<Bounded<Condition>> {
        let mut lhs = self.parse_unary()?;
        loop {
            self.skip_ws();
            if self.eat("&&") {
                self.skip_ws();
                let rhs = self.parse_unary()?;
                lhs = Bounded::parent(lhs.depth.max(rhs.depth), || lhs.value.and(rhs.value))?;
            } else {
                break;
            }
        }
        Ok(lhs)
    }

    fn parse_unary(&mut self) -> Result<Bounded<Condition>> {
        self.skip_ws();
        if self.eat("!") {
            self.skip_ws();
            let inner = self.nested(Self::parse_unary)?;
            return Bounded::parent(inner.depth, || inner.value.negate());
        }
        self.parse_primary()
    }

    /// Primary expression: a parenthesised expression, an Exists/Not_Exists
    /// check, a membership test, or a relational comparison.
    fn parse_primary(&mut self) -> Result<Bounded<Condition>> {
        self.skip_ws();
        if self.eat("(") {
            let e = self.nested(Self::parse_expr)?;
            self.skip_ws();
            if !self.eat(")") {
                return Err(CondError::Unbalanced("expected ')'"));
            }
            return Ok(e);
        }
        // Exists / Not_Exists
        if self.try_keyword("Exists") {
            self.skip_ws();
            let o = self.parse_operand()?;
            return Bounded::parent(o.depth, || Condition::Exists(o.value));
        }
        if self.try_keyword("Not_Exists") {
            self.skip_ws();
            let o = self.parse_operand()?;
            return Bounded::parent(o.depth, || Condition::NotExists(o.value));
        }

        // Membership operators — they're prefix in SDDL (`Member_of {…}`).
        for &(kw, op) in MEMBERSHIP_KEYWORDS {
            // Use a cloned parser-state save so the longest match wins.
            let saved = self.pos;
            if self.try_keyword(kw) {
                self.skip_ws();
                let o = self.parse_operand()?;
                return Bounded::parent(o.depth, || Condition::Member {
                    op,
                    operand: o.value,
                });
            }
            self.pos = saved;
        }

        // Otherwise: it's a relational comparison `operand op operand`.
        let lhs = self.parse_operand()?;
        self.skip_ws();
        let op = self.parse_compare_op()?;
        self.skip_ws();
        let rhs = self.parse_operand()?;
        Bounded::parent(lhs.depth.max(rhs.depth), || Condition::Compare {
            op,
            lhs: lhs.value,
            rhs: rhs.value,
        })
    }

    fn parse_compare_op(&mut self) -> Result<CompareOp> {
        // Order matters: "==" / "!=" / "<=" / ">=" before "<" / ">".
        for &(tok, op) in COMPARE_OPS {
            if self.eat(tok) {
                return Ok(op);
            }
        }
        // Word comparison ops.
        for &(kw, op) in COMPARE_KEYWORDS {
            let saved = self.pos;
            if self.try_keyword(kw) {
                return Ok(op);
            }
            self.pos = saved;
        }
        Err(CondError::UnknownToken(
            self.pos,
            self.rest().chars().take(8).collect(),
        ))
    }

    fn parse_operand(&mut self) -> Result<Bounded<Operand>> {
        self.skip_ws();
        if self.eof() {
            return Err(CondError::Eof);
        }
        let c = self.peek_char();
        match c {
            '"' => self.parse_string_lit().map(Operand::Str),
            '#' => self.parse_octet_lit().map(Operand::Octet),
            '{' => return self.nested(Self::parse_composite),
            '-' | '0'..='9' => self.parse_int_lit().map(Operand::Int),
            '@' => self.parse_attr_ref(),
            'S' | 's' => {
                if self.starts_with_ci("SID(") {
                    self.pos += 4;
                    let sid_str = self.take_until(')');
                    if !self.eat(")") {
                        return Err(CondError::Unbalanced("SID(... missing ')'"));
                    }
                    let sid = super::parse_sid_ref(&sid_str)
                        .map_err(|e| CondError::BadLiteral(self.pos, e.to_string()))?;
                    Ok(Operand::Sid(sid))
                } else {
                    Err(CondError::Unexpected(c, self.pos))
                }
            }
            _ => Err(CondError::Unexpected(c, self.pos)),
        }
        .map(Bounded::scalar)
    }

    fn parse_string_lit(&mut self) -> Result<String> {
        if !self.eat("\"") {
            return Err(CondError::Unexpected('"', self.pos));
        }
        let start = self.pos;
        let bytes = self.src.as_bytes();
        while self.pos < bytes.len() && bytes[self.pos] != b'"' {
            self.pos += 1;
        }
        if self.pos >= bytes.len() {
            return Err(CondError::Unbalanced("unterminated string literal"));
        }
        let s = self.src[start..self.pos].to_string();
        self.pos += 1; // closing quote
        Ok(s)
    }

    fn parse_octet_lit(&mut self) -> Result<Vec<u8>> {
        self.eat("#");
        let bytes = self.src.as_bytes();
        let start = self.pos;
        while self.pos < bytes.len() && bytes[self.pos].is_ascii_hexdigit() {
            self.pos += 1;
        }
        let hex = &self.src[start..self.pos];
        if hex.is_empty() {
            return Err(CondError::BadLiteral(self.pos, "empty octet".into()));
        }
        if !hex.len().is_multiple_of(2) {
            return Err(CondError::BadLiteral(
                self.pos,
                "odd hex digit count".into(),
            ));
        }
        let mut out = Vec::with_capacity(hex.len() / 2);
        let hex_bytes = hex.as_bytes();
        for i in (0..hex.len()).step_by(2) {
            let h = core::str::from_utf8(&hex_bytes[i..i + 2]).unwrap();
            out.push(
                u8::from_str_radix(h, 16)
                    .map_err(|_| CondError::BadLiteral(self.pos, "bad hex byte".into()))?,
            );
        }
        Ok(out)
    }

    fn parse_int_lit(&mut self) -> Result<i64> {
        let start = self.pos;
        if self.peek_char() == '-' {
            self.pos += 1;
        }
        // hex prefix?
        let after_sign = self.pos;
        let is_hex = self.starts_with("0x") || self.starts_with("0X");
        if is_hex {
            self.pos += 2;
            let dstart = self.pos;
            let bytes = self.src.as_bytes();
            while self.pos < bytes.len() && bytes[self.pos].is_ascii_hexdigit() {
                self.pos += 1;
            }
            if self.pos == dstart {
                return Err(CondError::BadLiteral(start, "empty hex literal".into()));
            }
            let lit = &self.src[start..self.pos];
            return super::parse_signed_i64(lit)
                .map_err(|_| CondError::BadLiteral(start, lit.to_string()));
        }
        // Decimal digits.
        let dstart = self.pos;
        let bytes = self.src.as_bytes();
        while self.pos < bytes.len() && bytes[self.pos].is_ascii_digit() {
            self.pos += 1;
        }
        if self.pos == dstart {
            self.pos = after_sign; // restore
            return Err(CondError::BadLiteral(start, "expected digit".into()));
        }
        let lit = &self.src[start..self.pos];
        super::parse_signed_i64(lit).map_err(|_| CondError::BadLiteral(start, lit.to_string()))
    }

    fn parse_composite(&mut self) -> Result<Bounded<Operand>> {
        if !self.eat("{") {
            return Err(CondError::Unexpected('{', self.pos));
        }
        let mut items: Vec<Operand> = Vec::new();
        let mut child_depth = 0;
        loop {
            self.skip_ws();
            if self.eat("}") {
                break;
            }
            let o = self.parse_operand()?;
            child_depth = child_depth.max(o.depth);
            items.push(o.value);
            self.skip_ws();
            if self.eat(",") {
                continue;
            } else if self.eat("}") {
                break;
            } else {
                return Err(CondError::Unbalanced("expected ',' or '}'"));
            }
        }
        Bounded::parent(child_depth, || Operand::Composite(items))
    }

    fn parse_attr_ref(&mut self) -> Result<Operand> {
        // @Local.Name | @User.Name | @Resource.Name | @Device.Name
        if !self.eat("@") {
            return Err(CondError::Unexpected('@', self.pos));
        }
        for &(kw, mk) in ATTR_PREFIXES {
            let saved = self.pos;
            if self.starts_with_ci(kw) {
                self.pos += kw.len();
                if !self.eat(".") {
                    self.pos = saved;
                    return Err(CondError::Unexpected('.', self.pos));
                }
                let name = self.parse_attr_name()?;
                return Ok(mk(name));
            }
            self.pos = saved;
        }
        Err(CondError::UnknownToken(
            self.pos,
            self.rest().chars().take(8).collect(),
        ))
    }

    fn parse_attr_name(&mut self) -> Result<String> {
        let bytes = self.src.as_bytes();
        let start = self.pos;
        while self.pos < bytes.len() && is_ident_continue(bytes[self.pos]) {
            self.pos += 1;
        }
        if self.pos == start {
            return Err(CondError::BadLiteral(
                self.pos,
                "empty attribute name".into(),
            ));
        }
        Ok(self.src[start..self.pos].to_string())
    }

    /// Consume up to (but not including) `end`. Used after `SID(` to grab
    /// the SID body.
    fn take_until(&mut self, end: char) -> String {
        let mut buf = String::new();
        let bytes = self.src.as_bytes();
        while self.pos < bytes.len() {
            let c = self.peek_char();
            if c == end {
                break;
            }
            let n = c.len_utf8();
            buf.push(c);
            self.pos += n;
        }
        buf
    }
}

fn is_ident_continue(b: u8) -> bool {
    // SDDL attribute names allow letters, digits, '_' and ':' (the spec
    // is permissive). We also accept '.' to allow dotted names.
    b.is_ascii_alphanumeric() || b == b'_' || b == b':' || b == b'.'
}

// ---------------------------------------------------------------------------
// Static tables (text-side)
// ---------------------------------------------------------------------------

const COMPARE_OPS: &[(&str, CompareOp)] = &[
    ("==", CompareOp::Eq),
    ("!=", CompareOp::Ne),
    ("<=", CompareOp::Le),
    (">=", CompareOp::Ge),
    ("<", CompareOp::Lt),
    (">", CompareOp::Gt),
];

const COMPARE_KEYWORDS: &[(&str, CompareOp)] = &[
    ("Contains", CompareOp::Contains),
    ("Any_of", CompareOp::AnyOf),
    ("Not_Contains", CompareOp::NotContains),
    ("Not_Any_of", CompareOp::NotAnyOf),
];

const MEMBERSHIP_KEYWORDS: &[(&str, MemberOp)] = &[
    // Longest keywords first so prefix matching doesn't shadow shorter
    // ones. `try_keyword` is boundary-aware so this is technically
    // redundant, but order keeps the table easy to read.
    ("Not_Device_Member_of_Any", MemberOp::NotDeviceMemberOfAny),
    ("Not_Member_of_Any", MemberOp::NotMemberOfAny),
    ("Not_Device_Member_of", MemberOp::NotDeviceMemberOf),
    ("Not_Member_of", MemberOp::NotMemberOf),
    ("Device_Member_of_Any", MemberOp::DeviceMemberOfAny),
    ("Device_Member_of", MemberOp::DeviceMemberOf),
    ("Member_of_Any", MemberOp::MemberOfAny),
    ("Member_of", MemberOp::MemberOf),
];

type AttrCtor = fn(String) -> Operand;
const ATTR_PREFIXES: &[(&str, AttrCtor)] = &[
    ("Local", Operand::Local as AttrCtor),
    ("User", Operand::User as AttrCtor),
    ("Resource", Operand::Resource as AttrCtor),
    ("Device", Operand::Device as AttrCtor),
];

// ---------------------------------------------------------------------------
// Formatter
// ---------------------------------------------------------------------------

/// `precedence` lets the printer decide whether to wrap a sub-expression
/// in parens — higher numbers bind tighter.
fn fmt_condition(c: &Condition, out: &mut String, parent_prec: u8) {
    match c {
        Condition::Or(a, b) => {
            let mine = 1u8;
            if parent_prec > mine {
                out.push('(');
            }
            fmt_condition(a, out, mine);
            out.push_str(" || ");
            fmt_condition(b, out, mine);
            if parent_prec > mine {
                out.push(')');
            }
        }
        Condition::And(a, b) => {
            let mine = 2u8;
            if parent_prec > mine {
                out.push('(');
            }
            fmt_condition(a, out, mine);
            out.push_str(" && ");
            fmt_condition(b, out, mine);
            if parent_prec > mine {
                out.push(')');
            }
        }
        Condition::Not(inner) => {
            out.push('!');
            fmt_condition(inner, out, 3);
        }
        Condition::Exists(o) => {
            out.push_str("Exists ");
            fmt_operand(o, out);
        }
        Condition::NotExists(o) => {
            out.push_str("Not_Exists ");
            fmt_operand(o, out);
        }
        Condition::Compare { op, lhs, rhs } => {
            fmt_operand(lhs, out);
            out.push(' ');
            out.push_str(compare_op_text(*op));
            out.push(' ');
            fmt_operand(rhs, out);
        }
        Condition::Member { op, operand } => {
            out.push_str(member_op_text(*op));
            out.push(' ');
            fmt_operand(operand, out);
        }
    }
}

fn fmt_operand(o: &Operand, out: &mut String) {
    match o {
        Operand::Int(n) => out.push_str(&format!("{n}")),
        Operand::Str(s) => {
            out.push('"');
            out.push_str(s);
            out.push('"');
        }
        Operand::Sid(sid) => {
            out.push_str("SID(");
            out.push_str(&super::format_sid(sid));
            out.push(')');
        }
        Operand::Octet(bytes) => {
            out.push('#');
            for b in bytes {
                out.push_str(&format!("{b:02x}"));
            }
        }
        Operand::Composite(items) => {
            out.push('{');
            for (i, it) in items.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                fmt_operand(it, out);
            }
            out.push('}');
        }
        Operand::Local(n) => {
            out.push_str("@Local.");
            out.push_str(n);
        }
        Operand::User(n) => {
            out.push_str("@User.");
            out.push_str(n);
        }
        Operand::Resource(n) => {
            out.push_str("@Resource.");
            out.push_str(n);
        }
        Operand::Device(n) => {
            out.push_str("@Device.");
            out.push_str(n);
        }
    }
}

fn compare_op_text(op: CompareOp) -> &'static str {
    match op {
        CompareOp::Eq => "==",
        CompareOp::Ne => "!=",
        CompareOp::Lt => "<",
        CompareOp::Le => "<=",
        CompareOp::Gt => ">",
        CompareOp::Ge => ">=",
        CompareOp::Contains => "Contains",
        CompareOp::AnyOf => "Any_of",
        CompareOp::NotContains => "Not_Contains",
        CompareOp::NotAnyOf => "Not_Any_of",
    }
}

fn member_op_text(op: MemberOp) -> &'static str {
    match op {
        MemberOp::MemberOf => "Member_of",
        MemberOp::DeviceMemberOf => "Device_Member_of",
        MemberOp::MemberOfAny => "Member_of_Any",
        MemberOp::DeviceMemberOfAny => "Device_Member_of_Any",
        MemberOp::NotMemberOf => "Not_Member_of",
        MemberOp::NotDeviceMemberOf => "Not_Device_Member_of",
        MemberOp::NotMemberOfAny => "Not_Member_of_Any",
        MemberOp::NotDeviceMemberOfAny => "Not_Device_Member_of_Any",
    }
}

// ---------------------------------------------------------------------------
// artx bytecode decoder
// ---------------------------------------------------------------------------

// Mirror of the bytecode alphabet in `crate::condition`. Kept private
// to this module because conditional bytecode is an implementation
// detail of the SDDL textual <-> binary bridge.
const TOK_INT64: u8 = 0x04;
const TOK_STRING: u8 = 0x10;
const TOK_OCTET: u8 = 0x18;
const TOK_COMPOSITE: u8 = 0x50;
const TOK_SID: u8 = 0x51;
const TOK_EXISTS: u8 = 0x87;
const TOK_NOT_EXISTS: u8 = 0x8d;
const TOK_AND: u8 = 0xa0;
const TOK_OR: u8 = 0xa1;
const TOK_NOT: u8 = 0xa2;
const TOK_ATTR_LOCAL: u8 = 0xf8;
const TOK_ATTR_USER: u8 = 0xf9;
const TOK_ATTR_RESOURCE: u8 = 0xfa;
const TOK_ATTR_DEVICE: u8 = 0xfb;

#[derive(Debug)]
enum Item {
    Operand(Bounded<Operand>),
    Condition(Bounded<Condition>),
}

struct ArtxDecoder<'a> {
    bytes: &'a [u8],
    pos: usize,
    stack: Vec<Item>,
}

impl<'a> ArtxDecoder<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            pos: 0,
            stack: Vec::new(),
        }
    }

    fn decode(&mut self) -> Result<Condition> {
        while self.pos < self.bytes.len() {
            let tok = self.bytes[self.pos];
            self.pos += 1;
            match tok {
                0x00 => {} // padding
                TOK_INT64 => {
                    let n = self.take_int64()?;
                    self.stack
                        .push(Item::Operand(Bounded::scalar(Operand::Int(n))));
                }
                TOK_STRING => {
                    let s = self.take_utf16_lp()?;
                    self.stack
                        .push(Item::Operand(Bounded::scalar(Operand::Str(s))));
                }
                TOK_OCTET => {
                    let bytes = self.take_lp_bytes()?;
                    self.stack
                        .push(Item::Operand(Bounded::scalar(Operand::Octet(bytes))));
                }
                TOK_SID => {
                    let bytes = self.take_lp_bytes()?;
                    let (sid, _) = crate::security::sddl::wire::SidRef::parse(&bytes)
                        .map_err(|_| CondError::Artx("malformed SID literal"))?;
                    self.stack
                        .push(Item::Operand(Bounded::scalar(Operand::Sid(sid.to_owned()))));
                }
                TOK_COMPOSITE => {
                    let body = self.take_lp_bytes()?;
                    let operand = ArtxDecoder::decode_composite(&body, 1)?;
                    self.stack.push(Item::Operand(operand));
                }
                TOK_ATTR_LOCAL => {
                    let n = self.take_utf16_lp()?;
                    self.stack
                        .push(Item::Operand(Bounded::scalar(Operand::Local(n))));
                }
                TOK_ATTR_USER => {
                    let n = self.take_utf16_lp()?;
                    self.stack
                        .push(Item::Operand(Bounded::scalar(Operand::User(n))));
                }
                TOK_ATTR_RESOURCE => {
                    let n = self.take_utf16_lp()?;
                    self.stack
                        .push(Item::Operand(Bounded::scalar(Operand::Resource(n))));
                }
                TOK_ATTR_DEVICE => {
                    let n = self.take_utf16_lp()?;
                    self.stack
                        .push(Item::Operand(Bounded::scalar(Operand::Device(n))));
                }
                TOK_EXISTS => self.push_unary_op(Condition::Exists)?,
                TOK_NOT_EXISTS => self.push_unary_op(Condition::NotExists)?,
                TOK_AND => self.push_binary_cond(Condition::And)?,
                TOK_OR => self.push_binary_cond(Condition::Or)?,
                TOK_NOT => {
                    let inner = self.pop_condition()?;
                    self.stack
                        .push(Item::Condition(Bounded::parent(inner.depth, || {
                            Condition::Not(Box::new(inner.value))
                        })?));
                }
                t => {
                    if let Some(op) = compare_from_byte(t) {
                        let rhs = self.pop_operand()?;
                        let lhs = self.pop_operand()?;
                        self.stack.push(Item::Condition(Bounded::parent(
                            lhs.depth.max(rhs.depth),
                            || Condition::Compare {
                                op,
                                lhs: lhs.value,
                                rhs: rhs.value,
                            },
                        )?));
                    } else if let Some(op) = member_from_byte(t) {
                        let operand = self.pop_operand()?;
                        self.stack
                            .push(Item::Condition(Bounded::parent(operand.depth, || {
                                Condition::Member {
                                    op,
                                    operand: operand.value,
                                }
                            })?));
                    } else {
                        return Err(CondError::Artx("unknown bytecode"));
                    }
                }
            }
        }
        if self.stack.len() != 1 {
            return Err(CondError::Artx(
                "stack did not collapse to single condition",
            ));
        }
        match self.stack.pop().unwrap() {
            Item::Condition(c) => Ok(c.value),
            Item::Operand(_) => Err(CondError::Artx(
                "top-level item was an operand, not a condition",
            )),
        }
    }

    fn push_unary_op<F: Fn(Operand) -> Condition>(&mut self, ctor: F) -> Result<()> {
        let o = self.pop_operand()?;
        self.stack
            .push(Item::Condition(Bounded::parent(o.depth, || ctor(o.value))?));
        Ok(())
    }

    fn push_binary_cond<F: Fn(Box<Condition>, Box<Condition>) -> Condition>(
        &mut self,
        ctor: F,
    ) -> Result<()> {
        let b = self.pop_condition()?;
        let a = self.pop_condition()?;
        self.stack.push(Item::Condition(Bounded::parent(
            a.depth.max(b.depth),
            || ctor(Box::new(a.value), Box::new(b.value)),
        )?));
        Ok(())
    }

    fn pop_operand(&mut self) -> Result<Bounded<Operand>> {
        match self.stack.pop() {
            Some(Item::Operand(o)) => Ok(o),
            Some(Item::Condition(_)) => Err(CondError::Artx("expected operand, got condition")),
            None => Err(CondError::Artx("operand stack underflow")),
        }
    }

    fn pop_condition(&mut self) -> Result<Bounded<Condition>> {
        match self.stack.pop() {
            Some(Item::Condition(c)) => Ok(c),
            Some(Item::Operand(_)) => Err(CondError::Artx("expected condition, got operand")),
            None => Err(CondError::Artx("condition stack underflow")),
        }
    }

    fn take_int64(&mut self) -> Result<i64> {
        // 8 LE bytes value + 1 sign byte + 1 base byte (we ignore base).
        let value_slice = self
            .bytes
            .get(self.pos..self.pos + 8)
            .ok_or(CondError::Artx("int64 truncated"))?;
        let n = i64::from_le_bytes([
            value_slice[0],
            value_slice[1],
            value_slice[2],
            value_slice[3],
            value_slice[4],
            value_slice[5],
            value_slice[6],
            value_slice[7],
        ]);
        self.pos += 8;
        // sign (1) + base (1) — skip both.
        if self.pos + 2 > self.bytes.len() {
            return Err(CondError::Artx("int64 metadata truncated"));
        }
        self.pos += 2;
        Ok(n)
    }

    fn take_lp_bytes(&mut self) -> Result<Vec<u8>> {
        if self.pos + 4 > self.bytes.len() {
            return Err(CondError::Artx("length-prefix truncated"));
        }
        let len = u32::from_le_bytes([
            self.bytes[self.pos],
            self.bytes[self.pos + 1],
            self.bytes[self.pos + 2],
            self.bytes[self.pos + 3],
        ]) as usize;
        self.pos += 4;
        let data = self
            .bytes
            .get(self.pos..self.pos + len)
            .ok_or(CondError::Artx("length-prefixed body out of bounds"))?
            .to_vec();
        self.pos += len;
        Ok(data)
    }

    fn take_utf16_lp(&mut self) -> Result<String> {
        let bytes = self.take_lp_bytes()?;
        if bytes.len() % 2 != 0 {
            return Err(CondError::Artx("UTF-16 length not even"));
        }
        let mut units = Vec::with_capacity(bytes.len() / 2);
        for i in (0..bytes.len()).step_by(2) {
            units.push(u16::from_le_bytes([bytes[i], bytes[i + 1]]));
        }
        String::from_utf16(&units).map_err(|_| CondError::Artx("malformed UTF-16 string"))
    }

    fn decode_composite(bytes: &[u8], nesting: usize) -> Result<Bounded<Operand>> {
        if nesting > MAX_CONDITION_DEPTH {
            return Err(CondError::TooDeep);
        }
        // A composite body is a sequence of operand tokens, no operators.
        let mut d = ArtxDecoder::new(bytes);
        let mut items: Vec<Operand> = Vec::new();
        let mut child_depth = 0;
        while d.pos < d.bytes.len() {
            let tok = d.bytes[d.pos];
            d.pos += 1;
            let o = match tok {
                0x00 => continue,
                TOK_INT64 => Operand::Int(d.take_int64()?),
                TOK_STRING => Operand::Str(d.take_utf16_lp()?),
                TOK_OCTET => Operand::Octet(d.take_lp_bytes()?),
                TOK_SID => {
                    let body = d.take_lp_bytes()?;
                    let (sid, _) = crate::security::sddl::wire::SidRef::parse(&body)
                        .map_err(|_| CondError::Artx("composite: malformed SID"))?;
                    Operand::Sid(sid.to_owned())
                }
                TOK_COMPOSITE => {
                    let inner = d.take_lp_bytes()?;
                    let operand = ArtxDecoder::decode_composite(&inner, nesting + 1)?;
                    child_depth = child_depth.max(operand.depth);
                    items.push(operand.value);
                    continue;
                }
                _ => return Err(CondError::Artx("composite contains non-operand token")),
            };
            items.push(o);
        }
        Bounded::parent(child_depth, || Operand::Composite(items))
    }
}

fn compare_from_byte(b: u8) -> Option<CompareOp> {
    match b {
        0x80 => Some(CompareOp::Eq),
        0x81 => Some(CompareOp::Ne),
        0x82 => Some(CompareOp::Lt),
        0x83 => Some(CompareOp::Le),
        0x84 => Some(CompareOp::Gt),
        0x85 => Some(CompareOp::Ge),
        0x86 => Some(CompareOp::Contains),
        0x88 => Some(CompareOp::AnyOf),
        0x8e => Some(CompareOp::NotContains),
        0x8f => Some(CompareOp::NotAnyOf),
        _ => None,
    }
}

fn member_from_byte(b: u8) -> Option<MemberOp> {
    match b {
        0x89 => Some(MemberOp::MemberOf),
        0x8a => Some(MemberOp::DeviceMemberOf),
        0x8b => Some(MemberOp::MemberOfAny),
        0x8c => Some(MemberOp::DeviceMemberOfAny),
        0x90 => Some(MemberOp::NotMemberOf),
        0x91 => Some(MemberOp::NotDeviceMemberOf),
        0x92 => Some(MemberOp::NotMemberOfAny),
        0x93 => Some(MemberOp::NotDeviceMemberOfAny),
        _ => None,
    }
}

#[cfg(test)]
mod depth_tests {
    use super::*;

    fn grouped(count: usize) -> String {
        format!("{}1 == 1{}", "(".repeat(count), ")".repeat(count))
    }

    fn composite_text(count: usize) -> String {
        format!(
            "Member_of {}SID(WD){}",
            "{".repeat(count),
            "}".repeat(count)
        )
    }

    fn composite_bytes(count: usize) -> Vec<u8> {
        let mut body = alloc::vec![TOK_INT64];
        body.extend_from_slice(&1i64.to_le_bytes());
        body.extend_from_slice(&[3, 2]);
        for _ in 0..count {
            let mut outer = alloc::vec![TOK_COMPOSITE];
            outer.extend_from_slice(&(body.len() as u32).to_le_bytes());
            outer.extend_from_slice(&body);
            body = outer;
        }
        let mut bytes = b"artx".to_vec();
        bytes.extend_from_slice(&body);
        bytes.push(0x89); // Member_of.
        bytes
    }

    fn boolean_bytes(terms: usize, operator: u8) -> Vec<u8> {
        let leaf = parse("1 == 1").unwrap().encode();
        let mut bytes = b"artx".to_vec();
        bytes.extend_from_slice(&leaf[4..]);
        for _ in 1..terms {
            bytes.extend_from_slice(&leaf[4..]);
            bytes.push(operator);
        }
        bytes
    }

    #[test]
    fn text_parentheses_have_a_128_level_recursion_limit() {
        assert!(parse(&grouped(MAX_CONDITION_DEPTH)).is_ok());
        for count in [MAX_CONDITION_DEPTH + 1, 16_000] {
            assert_eq!(parse(&grouped(count)), Err(CondError::TooDeep));
        }
    }

    #[test]
    fn text_negation_has_a_128_level_tree_limit() {
        let accepted = format!("{}1 == 1", "!".repeat(MAX_CONDITION_DEPTH - 1));
        assert!(parse(&accepted).is_ok());
        for count in [MAX_CONDITION_DEPTH, 16_000] {
            let rejected = format!("{}1 == 1", "!".repeat(count));
            assert_eq!(parse(&rejected), Err(CondError::TooDeep));
        }
    }

    #[test]
    fn text_boolean_chains_count_towards_tree_depth() {
        for operator in [" && ", " || "] {
            let accepted = alloc::vec!["1 == 1"; MAX_CONDITION_DEPTH].join(operator);
            assert!(parse(&accepted).is_ok());
            let rejected = format!("{accepted}{operator}1 == 1");
            assert_eq!(parse(&rejected), Err(CondError::TooDeep));
        }
    }

    #[test]
    fn text_composites_count_towards_tree_depth() {
        assert!(parse(&composite_text(MAX_CONDITION_DEPTH - 1)).is_ok());
        for count in [MAX_CONDITION_DEPTH, 16_000] {
            assert_eq!(parse(&composite_text(count)), Err(CondError::TooDeep));
        }
    }

    #[test]
    fn mixed_operators_and_operands_share_the_tree_limit() {
        let accepted = format!("!{}", composite_text(MAX_CONDITION_DEPTH - 2));
        let condition = parse(&accepted).unwrap();
        assert!(decode_artx(&condition.encode()).is_ok());
        assert_eq!(parse(&format!("!{accepted}")), Err(CondError::TooDeep));
        assert_eq!(
            parse(&format!("1 == 1 || {accepted}")),
            Err(CondError::TooDeep)
        );
    }

    #[test]
    fn delimiters_in_strings_do_not_consume_depth() {
        let literal = "(!{".repeat(1000);
        assert!(parse(&format!("@User.Name == \"{literal}\"")).is_ok());
    }

    #[test]
    fn binary_negation_has_a_128_level_tree_limit() {
        let mut bytes = parse("1 == 1").unwrap().encode();
        bytes.extend(core::iter::repeat_n(TOK_NOT, MAX_CONDITION_DEPTH - 1));
        assert!(decode_artx(&bytes).is_ok());
        bytes.push(TOK_NOT);
        assert_eq!(decode_artx(&bytes), Err(CondError::TooDeep));
        bytes.extend(core::iter::repeat_n(TOK_NOT, 16_000));
        assert_eq!(decode_artx(&bytes), Err(CondError::TooDeep));
    }

    #[test]
    fn binary_boolean_chains_count_towards_tree_depth() {
        for operator in [TOK_AND, TOK_OR] {
            assert!(decode_artx(&boolean_bytes(MAX_CONDITION_DEPTH, operator)).is_ok());
            for count in [MAX_CONDITION_DEPTH + 1, 16_000] {
                assert_eq!(
                    decode_artx(&boolean_bytes(count, operator)),
                    Err(CondError::TooDeep)
                );
            }
            // Delay all operators until every operand is on the decoder stack.
            let leaf = parse("1 == 1").unwrap().encode();
            for count in [MAX_CONDITION_DEPTH, MAX_CONDITION_DEPTH + 1] {
                let mut bytes = b"artx".to_vec();
                for _ in 0..count {
                    bytes.extend_from_slice(&leaf[4..]);
                }
                bytes.extend(core::iter::repeat_n(operator, count - 1));
                if count == MAX_CONDITION_DEPTH {
                    assert!(decode_artx(&bytes).is_ok());
                } else {
                    assert_eq!(decode_artx(&bytes), Err(CondError::TooDeep));
                }
            }
        }
    }

    #[test]
    fn binary_composites_count_towards_tree_depth() {
        assert!(decode_artx(&composite_bytes(MAX_CONDITION_DEPTH - 1)).is_ok());
        for count in [MAX_CONDITION_DEPTH, 1000] {
            assert_eq!(
                decode_artx(&composite_bytes(count)),
                Err(CondError::TooDeep)
            );
        }
    }

    #[test]
    fn truncated_input_drops_only_bounded_partial_trees() {
        let text = format!(
            "{} &&",
            alloc::vec!["1 == 1"; MAX_CONDITION_DEPTH].join(" && ")
        );
        assert!(parse(&text).is_err());
        let mut bytes = boolean_bytes(MAX_CONDITION_DEPTH, TOK_AND);
        bytes.push(TOK_COMPOSITE);
        assert!(decode_artx(&bytes).is_err());
    }
}
