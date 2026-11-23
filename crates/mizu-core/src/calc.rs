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

// ── Unit conversion: "10 km in mi", "70 f to c", "5 ft в см" ──

/// (aliases, kind, factor to the base unit). Temperature is handled separately.
const UNITS: &[(&[&str], &str, f64)] = &[
    (&["mm", "мм", "millimeter", "millimeters"], "len", 0.001),
    (&["cm", "см", "centimeter", "centimeters"], "len", 0.01),
    (&["m", "м", "meter", "meters", "metre", "metres", "метр", "метров", "метра"], "len", 1.0),
    (&["km", "км", "kilometer", "kilometers", "километр", "километров", "километра"], "len", 1000.0),
    (&["in", "inch", "inches", "дюйм", "дюймов", "дюйма"], "len", 0.0254),
    (&["ft", "foot", "feet", "фут", "футов", "фута"], "len", 0.3048),
    (&["yd", "yard", "yards", "ярд", "ярдов"], "len", 0.9144),
    (&["mi", "mile", "miles", "миля", "миль", "мили", "милях"], "len", 1609.344),
    (&["g", "г", "gram", "grams", "грамм", "граммов"], "mass", 0.001),
    (&["kg", "кг", "kilogram", "kilograms", "килограмм", "килограммов"], "mass", 1.0),
    (&["lb", "lbs", "pound", "pounds", "фунт", "фунтов", "фунта", "фунтах"], "mass", 0.453_592_37),
    (&["oz", "ounce", "ounces", "унция", "унций"], "mass", 0.028_349_523_125),
    (&["t", "т", "tonne", "tonnes", "тонна", "тонн"], "mass", 1000.0),
    (&["ml", "мл"], "vol", 0.001),
    (&["l", "л", "liter", "liters", "litre", "litres", "литр", "литров"], "vol", 1.0),
    (&["gal", "gallon", "gallons", "галлон", "галлонов"], "vol", 3.785_411_784),
    (&["kmh", "km/h", "kph", "км/ч"], "speed", 1.0 / 3.6),
    (&["mph"], "speed", 0.447_04),
    (&["m/s", "мс", "м/с"], "speed", 1.0),
    (&["kb", "кб"], "data", 1024.0),
    (&["mb", "мб"], "data", 1_048_576.0),
    (&["gb", "гб"], "data", 1_073_741_824.0),
    (&["tb", "тб"], "data", 1_099_511_627_776.0),
];

fn unit(name: &str) -> Option<(&'static str, f64, &'static str)> {
    let n = name.trim().trim_end_matches('.').to_lowercase();
    UNITS.iter().find(|(a, _, _)| a.contains(&n.as_str())).map(|(a, k, f)| (*k, *f, a[0]))
}

fn temp(name: &str) -> Option<&'static str> {
    match name.trim().trim_start_matches('°').to_lowercase().as_str() {
        "c" | "с" | "celsius" | "цельсия" | "цельсий" => Some("°C"),
        "f" | "fahrenheit" | "фаренгейта" | "фаренгейт" => Some("°F"),
        "k" | "к" | "kelvin" | "кельвина" | "кельвин" => Some("K"),
        _ => None,
    }
}

/// "10 km in mi" → "6.214 mi". Accepts "to", "in", "в", "=", "->".
pub fn convert(input: &str) -> Option<String> {
    let text = input.trim().to_lowercase().replace(',', ".");
    let (left, right) = ["->", " to ", " in ", " в ", " = "].iter().find_map(|sep| text.split_once(sep))?;
    let left = left.trim();
    let split = left.find(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-' || c == ' ')).unwrap_or(left.len());
    let value: f64 = left[..split].trim().parse().ok()?;
    let from = left[split..].trim();
    let to = right.trim();
    if from.is_empty() || to.is_empty() {
        return None;
    }
    if let (Some(a), Some(b)) = (temp(from), temp(to)) {
        let c = match a {
            "°F" => (value - 32.0) * 5.0 / 9.0,
            "K" => value - 273.15,
            _ => value,
        };
        let out = match b {
            "°F" => c * 9.0 / 5.0 + 32.0,
            "K" => c + 273.15,
            _ => c,
        };
        return Some(format!("{} {b}", round(out)));
    }
    let (ka, fa, _) = unit(from)?;
    let (kb, fb, name) = unit(to)?;
    (ka == kb).then(|| format!("{} {name}", round(value * fa / fb)))
}

fn round(v: f64) -> String {
    let r = if v.abs() >= 100.0 { (v * 100.0).round() / 100.0 } else { (v * 10_000.0).round() / 10_000.0 };
    format_number(r)
}

#[cfg(test)]
mod convert_tests {
    use super::convert;

    #[test]
    fn units() {
        assert_eq!(convert("10 km in mi").as_deref(), Some("6.2137 mi"));
        assert_eq!(convert("70 f to c").as_deref(), Some("21.1111 °C"));
        assert_eq!(convert("5 ft в см").as_deref(), Some("152.4 cm"));
        assert_eq!(convert("2 kg to lb").as_deref(), Some("4.4092 lb"));
        assert_eq!(convert("1,5 gb in mb").as_deref(), Some("1536 mb"));
        assert_eq!(convert("10 km in kg"), None);
        assert_eq!(convert("rust in action"), None);
    }
}
