use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Structured predicate for metadata filtering on vector points.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Filter {
    /// Field equals a given value.
    Eq(String, Value),
    /// Field does not equal a given value.
    Ne(String, Value),
    /// Field value is contained within a list of accepted values.
    In(String, Vec<Value>),
    /// Field value is not contained within a list of values.
    NotIn(String, Vec<Value>),
    /// Numerical field is strictly greater than.
    Gt(String, f64),
    /// Numerical field is greater than or equal to.
    Gte(String, f64),
    /// Numerical field is strictly less than.
    Lt(String, f64),
    /// Numerical field is less than or equal to.
    Lte(String, f64),
    /// String field contains a substring.
    Contains(String, String),
    /// Checks if a field exists and is not null.
    Exists(String),
    /// Logical AND of multiple sub-filters.
    And(Vec<Filter>),
    /// Logical OR of multiple sub-filters.
    Or(Vec<Filter>),
    /// Logical NOT negation of a sub-filter.
    Not(Box<Filter>),
}

impl Filter {
    /// Equality filter convenience helper.
    pub fn eq(field: impl Into<String>, value: impl Into<Value>) -> Self {
        Filter::Eq(field.into(), value.into())
    }

    /// Inequality filter convenience helper.
    pub fn ne(field: impl Into<String>, value: impl Into<Value>) -> Self {
        Filter::Ne(field.into(), value.into())
    }

    /// Inclusion filter convenience helper.
    pub fn in_vals(field: impl Into<String>, values: Vec<Value>) -> Self {
        Filter::In(field.into(), values)
    }

    /// Greater than filter convenience helper.
    pub fn gt(field: impl Into<String>, val: f64) -> Self {
        Filter::Gt(field.into(), val)
    }

    /// Greater than or equal to filter convenience helper.
    pub fn gte(field: impl Into<String>, val: f64) -> Self {
        Filter::Gte(field.into(), val)
    }

    /// Less than filter convenience helper.
    pub fn lt(field: impl Into<String>, val: f64) -> Self {
        Filter::Lt(field.into(), val)
    }

    /// Less than or equal to filter convenience helper.
    pub fn lte(field: impl Into<String>, val: f64) -> Self {
        Filter::Lte(field.into(), val)
    }

    /// String substring filter convenience helper.
    pub fn contains(field: impl Into<String>, substr: impl Into<String>) -> Self {
        Filter::Contains(field.into(), substr.into())
    }

    /// Logical AND helper.
    pub fn and(filters: Vec<Filter>) -> Self {
        Filter::And(filters)
    }

    /// Logical OR helper.
    pub fn or(filters: Vec<Filter>) -> Self {
        Filter::Or(filters)
    }

    /// Logical NOT helper.
    pub fn not(filter: Filter) -> Self {
        Filter::Not(Box::new(filter))
    }

    /// Evaluates if the filter matches a point's payload.
    pub fn matches(&self, payload: Option<&Value>) -> bool {
        match self {
            Filter::And(filters) => filters.iter().all(|f| f.matches(payload)),
            Filter::Or(filters) => filters.iter().any(|f| f.matches(payload)),
            Filter::Not(filter) => !filter.matches(payload),
            Filter::Exists(field) => {
                let Some(payload) = payload else { return false };
                resolve_path(payload, field).is_some_and(|v| !v.is_null())
            }
            Filter::Eq(field, expected) => {
                let Some(payload) = payload else { return false };
                resolve_path(payload, field) == Some(expected)
            }
            Filter::Ne(field, expected) => {
                let Some(payload) = payload else { return true };
                resolve_path(payload, field) != Some(expected)
            }
            Filter::In(field, accepted) => {
                let Some(payload) = payload else { return false };
                let val = resolve_path(payload, field);
                val.is_some_and(|v| accepted.contains(v))
            }
            Filter::NotIn(field, rejected) => {
                let Some(payload) = payload else { return true };
                let val = resolve_path(payload, field);
                match val {
                    Some(v) => !rejected.contains(v),
                    None => true,
                }
            }
            Filter::Gt(field, threshold) => {
                let Some(payload) = payload else { return false };
                resolve_number(payload, field).is_some_and(|num| num > *threshold)
            }
            Filter::Gte(field, threshold) => {
                let Some(payload) = payload else { return false };
                resolve_number(payload, field).is_some_and(|num| num >= *threshold)
            }
            Filter::Lt(field, threshold) => {
                let Some(payload) = payload else { return false };
                resolve_number(payload, field).is_some_and(|num| num < *threshold)
            }
            Filter::Lte(field, threshold) => {
                let Some(payload) = payload else { return false };
                resolve_number(payload, field).is_some_and(|num| num <= *threshold)
            }
            Filter::Contains(field, substr) => {
                let Some(payload) = payload else { return false };
                if let Some(Value::String(s)) = resolve_path(payload, field) {
                    s.contains(substr)
                } else {
                    false
                }
            }
        }
    }
}

/// Resolves a nested dot-separated path (e.g. `"user.profile.age"`) inside a JSON Value.
fn resolve_path<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
    let mut current = value;
    for part in path.split('.') {
        match current {
            Value::Object(map) => {
                current = map.get(part)?;
            }
            Value::Array(arr) => {
                let idx: usize = part.parse().ok()?;
                current = arr.get(idx)?;
            }
            _ => return None,
        }
    }
    Some(current)
}

/// Resolves a numerical value at the given path.
fn resolve_number(value: &Value, path: &str) -> Option<f64> {
    resolve_path(value, path).and_then(|v| {
        v.as_f64().or_else(|| v.as_i64().map(|i| i as f64))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_filter_evaluation() {
        let payload = json!({
            "title": "Vectrust High Performance Vector DB",
            "author": "Alice",
            "stars": 420,
            "category": "database",
            "tags": ["rust", "vector", "ann"],
            "meta": {
                "active": true,
                "version": 1.2
            }
        });

        assert!(Filter::eq("author", "Alice").matches(Some(&payload)));
        assert!(!Filter::eq("author", "Bob").matches(Some(&payload)));

        assert!(Filter::gt("stars", 400.0).matches(Some(&payload)));
        assert!(!Filter::gt("stars", 500.0).matches(Some(&payload)));

        assert!(Filter::gte("meta.version", 1.2).matches(Some(&payload)));

        assert!(Filter::contains("title", "High Performance").matches(Some(&payload)));

        // Logical AND
        let complex = Filter::and(vec![
            Filter::eq("category", "database"),
            Filter::gte("stars", 100.0),
            Filter::eq("meta.active", true),
        ]);
        assert!(complex.matches(Some(&payload)));
    }
}
