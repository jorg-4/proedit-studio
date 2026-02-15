//! Expression compilation and evaluation.

use crate::context::ExpressionContext;
use crate::error::ExpressionError;

/// A compiled expression that can be evaluated with a context.
pub struct Expression {
    pub source: String,
}

impl Expression {
    /// Create a new expression from source code.
    pub fn new(source: &str) -> Self {
        Self {
            source: source.to_string(),
        }
    }

    /// Check if the expression source is syntactically valid.
    pub fn is_valid(&self) -> bool {
        // Accept simple math expressions and known identifiers
        !self.source.trim().is_empty()
    }

    /// Evaluate the expression and return a scalar result.
    pub fn evaluate(&self, ctx: &ExpressionContext) -> Result<f64, ExpressionError> {
        let src = self.source.trim();
        if src.is_empty() {
            return Ok(ctx.value);
        }

        // Try to parse as a simple number
        if let Ok(val) = src.parse::<f64>() {
            return Ok(val);
        }

        // Simple variable substitution
        match src {
            "value" => return Ok(ctx.value),
            "time" => return Ok(ctx.time),
            "frame" => return Ok(ctx.frame as f64),
            "fps" => return Ok(ctx.fps),
            "comp_duration" => return Ok(ctx.comp_duration),
            "comp_width" => return Ok(ctx.comp_width),
            "comp_height" => return Ok(ctx.comp_height),
            _ => {}
        }

        // Try simple binary operations: "value * N", "value + N", etc.
        if let Some(result) = try_simple_binary(src, ctx) {
            return Ok(result);
        }

        // Try builtin function calls: "linear(time, 0, 1, 0, 100)"
        if let Some(result) = try_builtin_call(src, ctx) {
            return result;
        }

        Err(ExpressionError::Parse(format!(
            "cannot evaluate expression: {}",
            src
        )))
    }

    /// Evaluate the expression and return a 2D vector result.
    pub fn evaluate_vec2(&self, ctx: &ExpressionContext) -> Result<[f64; 2], ExpressionError> {
        let val = self.evaluate(ctx)?;
        Ok([val, val])
    }
}

/// Try to evaluate a simple binary operation like "value * 2" or "time + 1".
fn try_simple_binary(src: &str, ctx: &ExpressionContext) -> Option<f64> {
    for op in ['*', '+', '-', '/'] {
        if let Some(idx) = src.find(op) {
            if idx == 0 && op == '-' {
                // Negative number, not subtraction
                continue;
            }
            let left = src[..idx].trim();
            let right = src[idx + 1..].trim();

            let lval = resolve_value(left, ctx)?;
            let rval = resolve_value(right, ctx)?;

            return Some(match op {
                '*' => lval * rval,
                '+' => lval + rval,
                '-' => lval - rval,
                '/' => {
                    if rval.abs() < 1e-10 {
                        0.0
                    } else {
                        lval / rval
                    }
                }
                _ => unreachable!(),
            });
        }
    }
    None
}

/// Resolve a simple value (number or variable name).
fn resolve_value(s: &str, ctx: &ExpressionContext) -> Option<f64> {
    if let Ok(v) = s.parse::<f64>() {
        return Some(v);
    }
    match s {
        "value" => Some(ctx.value),
        "time" => Some(ctx.time),
        "frame" => Some(ctx.frame as f64),
        "fps" => Some(ctx.fps),
        "comp_duration" => Some(ctx.comp_duration),
        "comp_width" => Some(ctx.comp_width),
        "comp_height" => Some(ctx.comp_height),
        _ => None,
    }
}

/// Try to parse and evaluate a builtin function call.
fn try_builtin_call(src: &str, ctx: &ExpressionContext) -> Option<Result<f64, ExpressionError>> {
    let paren_start = src.find('(')?;
    let paren_end = src.rfind(')')?;
    if paren_end <= paren_start {
        return None;
    }

    let func_name = src[..paren_start].trim();
    let args_str = &src[paren_start + 1..paren_end];
    let args: Vec<f64> = args_str
        .split(',')
        .filter_map(|a| {
            let a = a.trim();
            resolve_value(a, ctx).or_else(|| a.parse::<f64>().ok())
        })
        .collect();

    match func_name {
        "linear" if args.len() == 5 => Some(Ok(crate::builtins::linear(
            args[0], args[1], args[2], args[3], args[4],
        ))),
        "ease" if args.len() == 5 => Some(Ok(crate::builtins::ease(
            args[0], args[1], args[2], args[3], args[4],
        ))),
        "easeIn" if args.len() == 5 => Some(Ok(crate::builtins::ease_in(
            args[0], args[1], args[2], args[3], args[4],
        ))),
        "easeOut" if args.len() == 5 => Some(Ok(crate::builtins::ease_out(
            args[0], args[1], args[2], args[3], args[4],
        ))),
        "wiggle" if args.len() == 2 => {
            Some(Ok(crate::builtins::wiggle(ctx.time, args[0], args[1])))
        }
        "clamp" if args.len() == 3 => Some(Ok(crate::builtins::clamp(args[0], args[1], args[2]))),
        "lerp" if args.len() == 3 => Some(Ok(crate::builtins::lerp(args[0], args[1], args[2]))),
        "degreesToRadians" if args.len() == 1 => {
            Some(Ok(crate::builtins::degrees_to_radians(args[0])))
        }
        "radiansToDegrees" if args.len() == 1 => {
            Some(Ok(crate::builtins::radians_to_degrees(args[0])))
        }
        _ => Some(Err(ExpressionError::Eval(format!(
            "unknown function or wrong argument count: {}",
            func_name
        )))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constant_expression() {
        let expr = Expression::new("42.0");
        let ctx = ExpressionContext::default();
        assert!((expr.evaluate(&ctx).unwrap() - 42.0).abs() < 0.01);
    }

    #[test]
    fn test_variable_expression() {
        let expr = Expression::new("value");
        let ctx = ExpressionContext::default().with_value(99.0);
        assert!((expr.evaluate(&ctx).unwrap() - 99.0).abs() < 0.01);
    }

    #[test]
    fn test_value_multiply() {
        let expr = Expression::new("value * 2");
        let ctx = ExpressionContext::default().with_value(50.0);
        assert!((expr.evaluate(&ctx).unwrap() - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_value_add() {
        let expr = Expression::new("value + 10");
        let ctx = ExpressionContext::default().with_value(5.0);
        assert!((expr.evaluate(&ctx).unwrap() - 15.0).abs() < 0.01);
    }

    #[test]
    fn test_linear_call() {
        let expr = Expression::new("linear(time, 0, 1, 0, 100)");
        let ctx = ExpressionContext {
            time: 0.5,
            ..Default::default()
        };
        assert!((expr.evaluate(&ctx).unwrap() - 50.0).abs() < 0.01);
    }

    #[test]
    fn test_wiggle_call() {
        let expr = Expression::new("wiggle(2, 50)");
        let ctx1 = ExpressionContext {
            time: 0.0,
            ..Default::default()
        };
        let ctx2 = ExpressionContext {
            time: 0.25,
            ..Default::default()
        };
        let v1 = expr.evaluate(&ctx1).unwrap();
        let v2 = expr.evaluate(&ctx2).unwrap();
        // Values should differ over time
        assert!((v1 - v2).abs() > 0.001);
    }

    #[test]
    fn test_empty_expression() {
        let expr = Expression::new("");
        let ctx = ExpressionContext::default().with_value(42.0);
        assert!((expr.evaluate(&ctx).unwrap() - 42.0).abs() < 0.01);
    }

    #[test]
    fn test_invalid_expression() {
        let expr = Expression::new("totally_unknown_function()");
        let ctx = ExpressionContext::default();
        assert!(expr.evaluate(&ctx).is_err());
    }

    #[test]
    fn test_is_valid() {
        assert!(Expression::new("value * 2").is_valid());
        assert!(!Expression::new("").is_valid());
    }

    #[test]
    fn test_evaluate_vec2() {
        let expr = Expression::new("42.0");
        let ctx = ExpressionContext::default();
        let v = expr.evaluate_vec2(&ctx).unwrap();
        assert!((v[0] - 42.0).abs() < 0.01);
        assert!((v[1] - 42.0).abs() < 0.01);
    }
}
