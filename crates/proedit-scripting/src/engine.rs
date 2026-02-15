//! Expression engine — manages expressions bound to properties.

use std::collections::HashMap;

use crate::context::ExpressionContext;
use crate::error::ExpressionError;
use crate::expression::Expression;

/// Expression engine that maps property paths to expressions.
pub struct ExpressionEngine {
    expressions: HashMap<String, Expression>,
}

impl ExpressionEngine {
    pub fn new() -> Self {
        Self {
            expressions: HashMap::new(),
        }
    }

    /// Set an expression for a property path.
    pub fn set_expression(&mut self, property: &str, source: &str) -> Result<(), ExpressionError> {
        let expr = Expression::new(source);
        if !expr.is_valid() {
            return Err(ExpressionError::Parse("empty expression".into()));
        }
        self.expressions.insert(property.to_string(), expr);
        Ok(())
    }

    /// Remove the expression for a property.
    pub fn remove_expression(&mut self, property: &str) {
        self.expressions.remove(property);
    }

    /// Evaluate the expression for a property, if one exists.
    pub fn evaluate(
        &self,
        property: &str,
        ctx: &ExpressionContext,
    ) -> Option<Result<f64, ExpressionError>> {
        self.expressions
            .get(property)
            .map(|expr| expr.evaluate(ctx))
    }

    /// Check if a property has an expression.
    pub fn has_expression(&self, property: &str) -> bool {
        self.expressions.contains_key(property)
    }

    /// Get all property paths that have expressions.
    pub fn all_properties(&self) -> Vec<&str> {
        self.expressions.keys().map(|s| s.as_str()).collect()
    }

    /// Number of active expressions.
    pub fn expression_count(&self) -> usize {
        self.expressions.len()
    }
}

impl Default for ExpressionEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_set_evaluate() {
        let mut engine = ExpressionEngine::new();
        engine
            .set_expression("transform.opacity", "value * 0.5")
            .unwrap();
        assert!(engine.has_expression("transform.opacity"));

        let ctx = ExpressionContext::default().with_value(100.0);
        let result = engine.evaluate("transform.opacity", &ctx).unwrap().unwrap();
        assert!((result - 50.0).abs() < 0.01);
    }

    #[test]
    fn test_engine_remove() {
        let mut engine = ExpressionEngine::new();
        engine.set_expression("position.x", "time * 100").unwrap();
        assert!(engine.has_expression("position.x"));
        engine.remove_expression("position.x");
        assert!(!engine.has_expression("position.x"));
    }

    #[test]
    fn test_engine_missing_property() {
        let engine = ExpressionEngine::new();
        let ctx = ExpressionContext::default();
        assert!(engine.evaluate("nonexistent", &ctx).is_none());
    }

    #[test]
    fn test_engine_all_properties() {
        let mut engine = ExpressionEngine::new();
        engine.set_expression("a", "1").unwrap();
        engine.set_expression("b", "2").unwrap();
        let props = engine.all_properties();
        assert_eq!(props.len(), 2);
    }

    #[test]
    fn test_engine_count() {
        let mut engine = ExpressionEngine::new();
        assert_eq!(engine.expression_count(), 0);
        engine.set_expression("x", "42").unwrap();
        assert_eq!(engine.expression_count(), 1);
    }

    #[test]
    fn test_set_empty_expression_fails() {
        let mut engine = ExpressionEngine::new();
        assert!(engine.set_expression("x", "").is_err());
    }
}
