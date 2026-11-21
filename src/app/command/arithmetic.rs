//! Arithmetic expansion, `$((expression))` (see ADR-0017).
//!
//! The expression is evaluated on 64-bit signed integers. Every operation is
//! checked: an overflow or a division by zero is an error, never a wrapped value
//! or a crash.

use crate::utils::shell_env;

/// Looks up the value of a variable by name.
pub(super) type Lookup<'a> = &'a dyn Fn(&str) -> Option<String>;

/// Evaluate `expression`. A variable is written `NAME` or `$NAME` (`${NAME}`); an
/// unset or empty one is `0`, and one that is not an integer is an error.
pub(super) fn evaluate(expression: &str, lookup: Lookup) -> Result<i64, String> {
    let mut parser = Parser {
        characters: expression.chars().collect(),
        position: 0,
        lookup,
        depth: 0,
    };
    let value = parser.logical_or()?;
    parser.skip_blanks();
    match parser.characters.get(parser.position) {
        None => Ok(value),
        Some(&character) => Err(format!("arithmetic: unexpected '{character}'")),
    }
}

/// Parentheses nested deeper than this are refused instead of overflowing the
/// stack on a hostile expression.
const MAX_DEPTH: usize = 64;

struct Parser<'a> {
    characters: Vec<char>,
    position: usize,
    lookup: Lookup<'a>,
    depth: usize,
}

impl Parser<'_> {
    fn skip_blanks(&mut self) {
        while self
            .characters
            .get(self.position)
            .is_some_and(|character| character.is_whitespace())
        {
            self.position += 1;
        }
    }

    /// Consume `operator` when it comes next. A one-character operator is not
    /// taken when it starts a longer one (`<` before `<=`, `&` before `&&`).
    fn eat(&mut self, operator: &str) -> bool {
        self.skip_blanks();
        let wanted: Vec<char> = operator.chars().collect();
        let end = self.position + wanted.len();
        if self.characters.get(self.position..end) != Some(wanted.as_slice()) {
            return false;
        }
        let next = self.characters.get(end).copied();
        let longer = match operator {
            "<" | ">" | "!" => next == Some('='),
            _ => false,
        };
        if longer {
            return false;
        }
        self.position = end;
        true
    }

    fn logical_or(&mut self) -> Result<i64, String> {
        let mut value = self.logical_and()?;
        while self.eat("||") {
            let right = self.logical_and()?;
            value = i64::from(value != 0 || right != 0);
        }
        Ok(value)
    }

    fn logical_and(&mut self) -> Result<i64, String> {
        let mut value = self.equality()?;
        while self.eat("&&") {
            let right = self.equality()?;
            value = i64::from(value != 0 && right != 0);
        }
        Ok(value)
    }

    fn equality(&mut self) -> Result<i64, String> {
        let mut value = self.comparison()?;
        loop {
            if self.eat("==") {
                value = i64::from(value == self.comparison()?);
            } else if self.eat("!=") {
                value = i64::from(value != self.comparison()?);
            } else {
                return Ok(value);
            }
        }
    }

    fn comparison(&mut self) -> Result<i64, String> {
        let mut value = self.additive()?;
        loop {
            if self.eat("<=") {
                value = i64::from(value <= self.additive()?);
            } else if self.eat(">=") {
                value = i64::from(value >= self.additive()?);
            } else if self.eat("<") {
                value = i64::from(value < self.additive()?);
            } else if self.eat(">") {
                value = i64::from(value > self.additive()?);
            } else {
                return Ok(value);
            }
        }
    }

    fn additive(&mut self) -> Result<i64, String> {
        let mut value = self.multiplicative()?;
        loop {
            if self.eat("+") {
                let right = self.multiplicative()?;
                value = value.checked_add(right).ok_or_else(overflow)?;
            } else if self.eat("-") {
                let right = self.multiplicative()?;
                value = value.checked_sub(right).ok_or_else(overflow)?;
            } else {
                return Ok(value);
            }
        }
    }

    fn multiplicative(&mut self) -> Result<i64, String> {
        let mut value = self.unary()?;
        loop {
            if self.eat("*") {
                let right = self.unary()?;
                value = value.checked_mul(right).ok_or_else(overflow)?;
            } else if self.eat("/") {
                let right = self.unary()?;
                if right == 0 {
                    return Err("arithmetic: division by 0".to_string());
                }
                value = value.checked_div(right).ok_or_else(overflow)?;
            } else if self.eat("%") {
                let right = self.unary()?;
                if right == 0 {
                    return Err("arithmetic: division by 0".to_string());
                }
                value = value.checked_rem(right).ok_or_else(overflow)?;
            } else {
                return Ok(value);
            }
        }
    }

    fn unary(&mut self) -> Result<i64, String> {
        // `-----…` recurses like parentheses do, so it shares their limit.
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            return Err("arithmetic: expression nested too deeply".to_string());
        }
        let value = if self.eat("-") {
            self.unary()?.checked_neg().ok_or_else(overflow)
        } else if self.eat("+") {
            self.unary()
        } else if self.eat("!") {
            self.unary().map(|operand| i64::from(operand == 0))
        } else {
            self.primary()
        };
        self.depth -= 1;
        value
    }

    fn primary(&mut self) -> Result<i64, String> {
        self.skip_blanks();
        let Some(&character) = self.characters.get(self.position) else {
            return Err("arithmetic: missing operand".to_string());
        };
        if character == '(' {
            self.position += 1;
            self.depth += 1;
            if self.depth > MAX_DEPTH {
                return Err("arithmetic: expression nested too deeply".to_string());
            }
            let value = self.logical_or()?;
            if !self.eat(")") {
                return Err("arithmetic: missing ')'".to_string());
            }
            self.depth -= 1;
            return Ok(value);
        }
        if character.is_ascii_digit() {
            return self.number();
        }
        if character == '$' {
            self.position += 1;
            return self.variable(true);
        }
        if character.is_ascii_alphabetic() || character == '_' {
            return self.variable(false);
        }
        Err(format!("arithmetic: unexpected '{character}'"))
    }

    fn number(&mut self) -> Result<i64, String> {
        let start = self.position;
        while self
            .characters
            .get(self.position)
            .is_some_and(|character| character.is_ascii_alphanumeric())
        {
            self.position += 1;
        }
        let text: String = self.characters[start..self.position].iter().collect();
        let parsed = match text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
            Some(hex) => i64::from_str_radix(hex, 16),
            None => text.parse::<i64>(),
        };
        parsed.map_err(|_| format!("arithmetic: invalid number '{text}'"))
    }

    /// A variable after its optional `$`: `NAME`, `{NAME}` or, with the dollar, `?`.
    fn variable(&mut self, dollar: bool) -> Result<i64, String> {
        let braced = dollar && self.characters.get(self.position) == Some(&'{');
        if braced {
            self.position += 1;
        }
        if dollar && self.characters.get(self.position) == Some(&'(') {
            return Err("arithmetic: command substitution is not supported here".to_string());
        }
        let start = self.position;
        if dollar && self.characters.get(start) == Some(&'?') {
            self.position += 1;
        } else {
            while self
                .characters
                .get(self.position)
                .is_some_and(|character| character.is_ascii_alphanumeric() || *character == '_')
            {
                self.position += 1;
            }
        }
        let name: String = self.characters[start..self.position].iter().collect();
        if braced {
            if self.characters.get(self.position) != Some(&'}') {
                return Err("arithmetic: missing '}'".to_string());
            }
            self.position += 1;
        }
        if name != "?" && !shell_env::is_valid_name(&name) {
            return Err(format!("arithmetic: invalid variable name '{name}'"));
        }
        match (self.lookup)(&name).as_deref().map(str::trim) {
            None | Some("") => Ok(0),
            Some(text) => text
                .parse::<i64>()
                .map_err(|_| format!("arithmetic: {name}: not an integer: '{text}'")),
        }
    }
}

fn overflow() -> String {
    "arithmetic: result out of range".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn variables(name: &str) -> Option<String> {
        match name {
            "N" => Some("7".to_string()),
            "NEG" => Some("-3".to_string()),
            "EMPTY" => Some(String::new()),
            "TEXT" => Some("abc".to_string()),
            "?" => Some("2".to_string()),
            _ => None,
        }
    }

    fn calc(expression: &str) -> Result<i64, String> {
        evaluate(expression, &variables)
    }

    #[test]
    fn the_four_operations_and_the_remainder() {
        assert_eq!(calc("1+2"), Ok(3));
        assert_eq!(calc("10 - 4"), Ok(6));
        assert_eq!(calc("6*7"), Ok(42));
        assert_eq!(calc("7/2"), Ok(3));
        assert_eq!(calc("-7/2"), Ok(-3));
        assert_eq!(calc("7%3"), Ok(1));
        assert_eq!(calc("-7%3"), Ok(-1));
    }

    #[test]
    fn multiplication_binds_tighter_and_parentheses_group() {
        assert_eq!(calc("2+3*4"), Ok(14));
        assert_eq!(calc("(2+3)*4"), Ok(20));
        assert_eq!(calc("10-2-3"), Ok(5));
        assert_eq!(calc("100/10/5"), Ok(2));
        assert_eq!(calc("((1+1))*((2))"), Ok(4));
    }

    #[test]
    fn unary_signs_and_negation() {
        assert_eq!(calc("-5"), Ok(-5));
        assert_eq!(calc("--5"), Ok(5));
        assert_eq!(calc("+5"), Ok(5));
        assert_eq!(calc("3 - -2"), Ok(5));
        assert_eq!(calc("!0"), Ok(1));
        assert_eq!(calc("!5"), Ok(0));
    }

    #[test]
    fn comparisons_and_logic_give_zero_or_one() {
        assert_eq!(calc("3<4"), Ok(1));
        assert_eq!(calc("4<=4"), Ok(1));
        assert_eq!(calc("5>6"), Ok(0));
        assert_eq!(calc("6>=7"), Ok(0));
        assert_eq!(calc("2==2"), Ok(1));
        assert_eq!(calc("2!=2"), Ok(0));
        assert_eq!(calc("1&&0"), Ok(0));
        assert_eq!(calc("1||0"), Ok(1));
        assert_eq!(calc("0||0"), Ok(0));
        assert_eq!(calc("1+1==2&&3>2"), Ok(1));
    }

    #[test]
    fn variables_by_name_with_a_dollar_or_with_braces() {
        assert_eq!(calc("N+1"), Ok(8));
        assert_eq!(calc("$N*2"), Ok(14));
        assert_eq!(calc("${N}%4"), Ok(3));
        assert_eq!(calc("NEG*2"), Ok(-6));
        assert_eq!(calc("$?+1"), Ok(3));
    }

    #[test]
    fn an_unset_or_empty_variable_is_zero_and_text_is_an_error() {
        assert_eq!(calc("MISSING+1"), Ok(1));
        assert_eq!(calc("EMPTY+1"), Ok(1));
        assert_eq!(
            calc("TEXT+1"),
            Err("arithmetic: TEXT: not an integer: 'abc'".to_string())
        );
    }

    #[test]
    fn numbers_may_be_hexadecimal() {
        assert_eq!(calc("0x10"), Ok(16));
        assert_eq!(calc("0XfF"), Ok(255));
        assert!(calc("0xZZ").is_err());
        assert!(calc("12ab").is_err());
    }

    #[test]
    fn a_division_by_zero_and_an_overflow_are_errors() {
        assert_eq!(calc("1/0"), Err("arithmetic: division by 0".to_string()));
        assert_eq!(calc("1%0"), Err("arithmetic: division by 0".to_string()));
        assert!(calc("9223372036854775807+1").is_err());
        assert!(calc("-9223372036854775807-2").is_err());
        assert!(calc("9223372036854775807*2").is_err());
        assert!(calc("(-9223372036854775807-1)/-1").is_err());
        assert!(calc("-(-9223372036854775807-1)").is_err());
        assert_eq!(calc("9223372036854775807"), Ok(i64::MAX));
    }

    #[test]
    fn mistakes_are_reported() {
        assert!(calc("").unwrap_err().contains("missing operand"));
        assert!(calc("1+").unwrap_err().contains("missing operand"));
        assert!(calc("(1+2").unwrap_err().contains("missing ')'"));
        assert!(calc("1 2").unwrap_err().contains("unexpected '2'"));
        assert!(calc("1+*2").unwrap_err().contains("unexpected '*'"));
        assert!(calc("1=2").unwrap_err().contains("unexpected '='"));
        assert!(calc("$(date)").unwrap_err().contains("not supported"));
        assert!(calc("${N").unwrap_err().contains("missing '}'"));
        assert!(calc("$").unwrap_err().contains("invalid variable name"));
    }

    #[test]
    fn deeply_nested_parentheses_are_refused_without_a_crash() {
        let expression = format!("{}1{}", "(".repeat(5000), ")".repeat(5000));
        assert!(calc(&expression).unwrap_err().contains("nested too deeply"));
        let signs = format!("{}1", "-".repeat(100_000));
        assert!(calc(&signs).unwrap_err().contains("nested too deeply"));
        // A moderate nesting still works.
        assert_eq!(
            calc(&format!("{}1{}", "(".repeat(20), ")".repeat(20))),
            Ok(1)
        );
    }

    /// Whatever the expression, the evaluator answers with a value or an error and
    /// never panics (a wrapped or unchecked operation would panic in a debug build).
    #[test]
    fn random_expressions_never_panic() {
        const PIECES: [&str; 28] = [
            "0",
            "1",
            "7",
            "9223372036854775807",
            "9223372036854775808",
            "0x",
            "0xFF",
            "N",
            "$N",
            "${N}",
            "$?",
            "$",
            "(",
            ")",
            "+",
            "-",
            "*",
            "/",
            "%",
            "<",
            "<=",
            "==",
            "!=",
            "&&",
            "||",
            "!",
            " ",
            "=",
        ];
        let mut seed = 0xA076_1D64_78BD_642F_u64;
        let mut next = move || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed
        };
        for _ in 0..200_000 {
            let length = 1 + (next() % 12) as usize;
            let text: String = (0..length)
                .map(|_| PIECES[(next() % PIECES.len() as u64) as usize])
                .collect();
            let outcome = std::panic::catch_unwind(|| {
                let _ = calc(&text);
            });
            assert!(outcome.is_ok(), "the evaluator panicked on {text:?}");
        }
    }
}
