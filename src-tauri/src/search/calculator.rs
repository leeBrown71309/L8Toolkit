//! Inline calculator: a tiny recursive-descent parser for arithmetic typed in
//! the search field ("12*4+3", "sqrt(2)", "2^10", "3,5*2").
//!
//! Grammar:
//!   expression := term (('+' | '-') term)*
//!   term       := unary (('*' | '/' | '%') unary)*
//!   unary      := ('+' | '-') unary | power
//!   power      := primary ('^' unary)?
//!   primary    := number | constant | function '(' expression ')' | '(' expression ')'

const MAX_INPUT_LENGTH: usize = 200;
const DISPLAY_DECIMALS: usize = 10;
const INTEGER_DISPLAY_LIMIT: f64 = 1e15;

/// Evaluates `input` when it is a real calculation. Plain numbers ("2024")
/// and words return `None` so the calculator never hijacks normal searches.
pub fn evaluate(input: &str) -> Option<f64> {
    let trimmed = input.trim().trim_start_matches('=').trim();
    if trimmed.is_empty() || trimmed.len() > MAX_INPUT_LENGTH {
        return None;
    }
    if !trimmed.chars().any(|character| character.is_ascii_digit()) {
        return None;
    }
    // Accept a comma as decimal separator ("3,5") when no dot is present.
    let normalized = if trimmed.contains('.') { trimmed.to_string() } else { trimmed.replace(',', ".") };

    let mut parser = Parser::new(&normalized);
    let value = parser.expression()?;
    parser.skip_whitespace();
    let fully_consumed = parser.position == parser.characters.len();
    (fully_consumed && parser.performed_operation && value.is_finite()).then_some(value)
}

/// Human-friendly rendering: integers without decimals, others trimmed.
pub fn format_number(value: f64) -> String {
    if value.fract() == 0.0 && value.abs() < INTEGER_DISPLAY_LIMIT {
        return format!("{value:.0}");
    }
    let fixed = format!("{value:.DISPLAY_DECIMALS$}");
    let trimmed = fixed.trim_end_matches('0').trim_end_matches('.');
    if trimmed == "-0" { "0".to_string() } else { trimmed.to_string() }
}

struct Parser {
    characters: Vec<char>,
    position: usize,
    /// True once an operator or function ran; distinguishes "2" from "2+0".
    performed_operation: bool,
}

impl Parser {
    fn new(input: &str) -> Self {
        Self { characters: input.chars().collect(), position: 0, performed_operation: false }
    }

    fn skip_whitespace(&mut self) {
        while self.peek().is_some_and(char::is_whitespace) {
            self.position += 1;
        }
    }

    fn peek(&self) -> Option<char> {
        self.characters.get(self.position).copied()
    }

    fn consume(&mut self, expected: char) -> bool {
        self.skip_whitespace();
        if self.peek() == Some(expected) {
            self.position += 1;
            true
        } else {
            false
        }
    }

    fn expression(&mut self) -> Option<f64> {
        let mut value = self.term()?;
        loop {
            if self.consume('+') {
                value += self.term()?;
            } else if self.consume('-') {
                value -= self.term()?;
            } else {
                return Some(value);
            }
            self.performed_operation = true;
        }
    }

    fn term(&mut self) -> Option<f64> {
        let mut value = self.unary()?;
        loop {
            if self.consume('*') || self.consume('×') {
                value *= self.unary()?;
            } else if self.consume('/') || self.consume('÷') {
                let divisor = self.unary()?;
                if divisor == 0.0 {
                    return None;
                }
                value /= divisor;
            } else if self.consume('%') {
                let divisor = self.unary()?;
                if divisor == 0.0 {
                    return None;
                }
                value %= divisor;
            } else {
                return Some(value);
            }
            self.performed_operation = true;
        }
    }

    fn unary(&mut self) -> Option<f64> {
        if self.consume('-') {
            return self.unary().map(|value| -value);
        }
        if self.consume('+') {
            return self.unary();
        }
        self.power()
    }

    fn power(&mut self) -> Option<f64> {
        let base = self.primary()?;
        if self.consume('^') {
            self.performed_operation = true;
            let exponent = self.unary()?;
            return Some(base.powf(exponent));
        }
        Some(base)
    }

    fn primary(&mut self) -> Option<f64> {
        self.skip_whitespace();
        let current = self.peek()?;
        if current == '(' {
            self.position += 1;
            let value = self.expression()?;
            return self.consume(')').then_some(value);
        }
        if current.is_ascii_digit() || current == '.' {
            return self.number();
        }
        if current.is_ascii_alphabetic() {
            return self.identifier();
        }
        None
    }

    fn number(&mut self) -> Option<f64> {
        let start = self.position;
        while self.peek().is_some_and(|character| character.is_ascii_digit() || character == '.') {
            self.position += 1;
        }
        let literal: String = self.characters[start..self.position].iter().collect();
        literal.parse().ok()
    }

    fn identifier(&mut self) -> Option<f64> {
        let start = self.position;
        while self.peek().is_some_and(|character| character.is_ascii_alphabetic()) {
            self.position += 1;
        }
        let name: String = self.characters[start..self.position].iter().collect::<String>().to_lowercase();
        match name.as_str() {
            "pi" => return Some(std::f64::consts::PI),
            "e" => return Some(std::f64::consts::E),
            _ => {}
        }

        if !self.consume('(') {
            return None;
        }
        let argument = self.expression()?;
        if !self.consume(')') {
            return None;
        }
        self.performed_operation = true;
        let result = match name.as_str() {
            "sqrt" => argument.sqrt(),
            "abs" => argument.abs(),
            "sin" => argument.sin(),
            "cos" => argument.cos(),
            "tan" => argument.tan(),
            "ln" => argument.ln(),
            "log" => argument.log10(),
            "round" => argument.round(),
            "floor" => argument.floor(),
            "ceil" => argument.ceil(),
            _ => return None,
        };
        Some(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn respects_operator_precedence() {
        assert_eq!(evaluate("2+3*4"), Some(14.0));
        assert_eq!(evaluate("(2+3)*4"), Some(20.0));
        assert_eq!(evaluate("2^3^2"), Some(512.0));
        assert_eq!(evaluate("-2^2"), Some(-4.0));
        assert_eq!(evaluate("10 % 4"), Some(2.0));
    }

    #[test]
    fn supports_functions_constants_and_comma_decimals() {
        assert_eq!(evaluate("sqrt(16)"), Some(4.0));
        assert_eq!(evaluate("3,5*2"), Some(7.0));
        assert_eq!(evaluate("= 7 / 2"), Some(3.5));
        assert!((evaluate("2*pi").unwrap() - std::f64::consts::TAU).abs() < 1e-12);
    }

    #[test]
    fn ignores_non_calculations() {
        assert_eq!(evaluate("2024"), None);
        assert_eq!(evaluate("-5"), None);
        assert_eq!(evaluate("chrome"), None);
        assert_eq!(evaluate("e"), None);
        assert_eq!(evaluate("2+"), None);
        assert_eq!(evaluate("1/0"), None);
        assert_eq!(evaluate("unknown(2)"), None);
    }

    #[test]
    fn formats_numbers_for_display() {
        assert_eq!(format_number(42.0), "42");
        assert_eq!(format_number(3.5), "3.5");
        assert_eq!(format_number(1.0 / 3.0), "0.3333333333");
        assert_eq!(format_number(-0.00000000001), "0");
    }
}
