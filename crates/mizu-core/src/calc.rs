//! Address-bar calculator: `2+2*3`, `(1,5+2)^2`, `15% * 80`. Only digits, operators and
//! parentheses are accepted, so nothing a user types is ever executed.

/// Evaluates `input` when it is clearly arithmetic (at least one operator between
/// numbers). Returns the result formatted for display, or `None`.
pub fn evaluate(input: &str) -> Option<String> {
    let text = input.trim().trim_end_matches('=').trim();
    if text.is_empty() || text.len() > 200 {
        return None;
    }
    if !text.chars().all(|c| c.is_ascii_digit() || " .,+-*/x×÷^%()".contains(c)) {
        return None;
    }
    // A lone number (or a date-like "2024-11-15" with no spaces is still maths, but a
    // plain "42" or "-3" is not worth a suggestion).
    let mut p = Parser { s: text.as_bytes(), i: 0, ops: 0 };
    let v = p.expr()?;
    p.skip_ws();
    if p.i != p.s.len() || p.ops == 0 || !v.is_finite() {
        return None;
    }
    Some(format_number(v))
}

fn format_number(v: f64) -> String {
    if v == v.trunc() && v.abs() < 1e15 {
        return format!("{}", v as i64);
    }
    let s = format!("{v:.10}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    s.to_owned()
}

struct Parser<'a> {
    s: &'a [u8],
    i: usize,
    ops: usize,
}

impl Parser<'_> {
    fn skip_ws(&mut self) {
        while self.s.get(self.i) == Some(&b' ') {
            self.i += 1;
        }
    }

    fn peek(&mut self) -> Option<char> {
        self.skip_ws();
        let rest = std::str::from_utf8(&self.s[self.i..]).ok()?;
        rest.chars().next()
    }

    fn bump(&mut self, c: char) {
        self.i += c.len_utf8();
    }

    // expr := term (('+' | '-') term)*
    fn expr(&mut self) -> Option<f64> {
        let mut v = self.term()?;
        while let Some(c @ ('+' | '-')) = self.peek() {
            self.bump(c);
            self.ops += 1;
            let r = self.term()?;
            v = if c == '+' { v + r } else { v - r };
        }
        Some(v)
    }

    // term := power (('*' | 'x' | '×' | '/' | '÷') power)*
    fn term(&mut self) -> Option<f64> {
        let mut v = self.power()?;
        while let Some(c @ ('*' | 'x' | '×' | '/' | '÷')) = self.peek() {
            self.bump(c);
            self.ops += 1;
            let r = self.power()?;
            v = if matches!(c, '/' | '÷') {
                if r == 0.0 {
                    return None;
                }
                v / r
            } else {
                v * r
            };
        }
        Some(v)
    }

    // power := unary ('^' power)?
    fn power(&mut self) -> Option<f64> {
        let base = self.unary()?;
        if self.peek() == Some('^') {
            self.bump('^');
            self.ops += 1;
            let exp = self.power()?;
            return Some(base.powf(exp));
        }
        Some(base)
    }

    // unary := '-' unary | atom '%'?
    fn unary(&mut self) -> Option<f64> {
        match self.peek()? {
            '-' => {
                self.bump('-');
                Some(-self.unary()?)
            }
            '+' => {
                self.bump('+');
                self.unary()
            }
            _ => {
                let v = self.atom()?;
                if self.peek() == Some('%') {
                    self.bump('%');
                    self.ops += 1;
                    return Some(v / 100.0);
                }
                Some(v)
            }
        }
    }

    fn atom(&mut self) -> Option<f64> {
        if self.peek()? == '(' {
            self.bump('(');
            let v = self.expr()?;
            if self.peek()? != ')' {
                return None;
            }
            self.bump(')');
            return Some(v);
        }
        self.skip_ws();
        let start = self.i;
        let mut seen_sep = false;
        while let Some(&b) = self.s.get(self.i) {
            if b.is_ascii_digit() {
                self.i += 1;
            } else if (b == b'.' || b == b',') && !seen_sep {
                seen_sep = true;
                self.i += 1;
            } else {
                break;
            }
        }
        if self.i == start {
            return None;
        }
        std::str::from_utf8(&self.s[start..self.i]).ok()?.replace(',', ".").parse().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::evaluate;

    #[test]
    fn arithmetic() {
        assert_eq!(evaluate("2+2*3").as_deref(), Some("8"));
        assert_eq!(evaluate("(1,5 + 2)^2").as_deref(), Some("12.25"));
        assert_eq!(evaluate("10 / 4 =").as_deref(), Some("2.5"));
        assert_eq!(evaluate("15% * 80").as_deref(), Some("12"));
        assert_eq!(evaluate("-3 * -3").as_deref(), Some("9"));
        assert_eq!(evaluate("2^10").as_deref(), Some("1024"));
        assert_eq!(evaluate("7 × 6").as_deref(), Some("42"));
    }

    #[test]
    fn not_maths() {
        for s in ["42", "-3", "1.2.3.4", "example.com", "rust 2+2", "1/0", "2+", "", "(1+2"] {
            assert_eq!(evaluate(s), None, "{s}");
        }
    }
}
