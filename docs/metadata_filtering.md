# Metadata Filtering in Vectrust

Real-world vector applications almost always require combining semantic similarity with structured attribute constraints (e.g. "Find documents similar to query X where `organization == 'Tech'` AND `created_at >= 2024-01-01`").

Vectrust features an embedded metadata evaluation engine supporting nested JSON paths and boolean logic.

---

## 1. Filter Operators Reference

| Operator | Rust Constructor | Description | Example |
| :--- | :--- | :--- | :--- |
| **Equality** | `Filter::eq(path, value)` | Exact match against a JSON value | `Filter::eq("author", "Alice")` |
| **Inequality** | `Filter::ne(path, value)` | Value does not equal the target | `Filter::ne("status", "draft")` |
| **Inclusion** | `Filter::in_vals(path, vec)` | Value matches one of the array elements | `Filter::in_vals("tag", vec![json!("rust"), json!("ai")])` |
| **Exclusion** | `Filter::NotIn(path, vec)` | Value does not match any element in array | `Filter::NotIn("env", vec![json!("staging")])` |
| **Greater Than** | `Filter::gt(path, float)` | Numerical field strictly greater than | `Filter::gt("price", 99.99)` |
| **Greater / Equal** | `Filter::gte(path, float)` | Numerical field greater than or equal to | `Filter::gte("rating", 4.5)` |
| **Less Than** | `Filter::lt(path, float)` | Numerical field strictly less than | `Filter::lt("age", 30.0)` |
| **Less / Equal** | `Filter::lte(path, float)` | Numerical field less than or equal to | `Filter::lte("stock", 0.0)` |
| **Substring** | `Filter::contains(path, str)` | String field contains substring | `Filter::contains("title", "Vector")` |
| **Field Exists** | `Filter::Exists(path)` | Field exists and is non-null | `Filter::Exists("email")` |
| **Logical AND** | `Filter::and(vec)` | All sub-predicates must evaluate to true | `Filter::and(vec![f1, f2])` |
| **Logical OR** | `Filter::or(vec)` | At least one sub-predicate must be true | `Filter::or(vec![f1, f2])` |
| **Logical NOT** | `Filter::not(filter)` | Inverts the evaluation of sub-predicate | `Filter::not(f1)` |

---

## 2. Nested JSON Paths

Vectrust supports traversing deeply nested JSON structures using standard dot notation:

```json
{
  "product": {
    "specifications": {
      "ram_gb": 32,
      "cpu": "arm"
    }
  }
}
```

Path query:
```rust
Filter::gte("product.specifications.ram_gb", 16.0);
Filter::eq("product.specifications.cpu", "arm");
```

---

## 3. Complex Boolean Filter Examples

### Example: E-Commerce Product Search
Filter for laptops with at least 16GB RAM that are in-stock and cost under $2000, OR any discounted item from Apple:

```rust
let filter = Filter::or(vec![
    // Condition 1: High-spec affordable laptop
    Filter::and(vec![
        Filter::eq("category", "laptop"),
        Filter::gte("specs.ram", 16.0),
        Filter::eq("in_stock", true),
        Filter::lte("price", 2000.00),
    ]),
    // Condition 2: Discounted Apple product
    Filter::and(vec![
        Filter::eq("brand", "Apple"),
        Filter::eq("on_sale", true),
    ]),
]);

let query = SearchQuery::new(embedding, 10).with_filter(filter);
let results = collection.search(query)?;
```

---

## 4. Performance & Graph Traversal Strategy

Vectrust implements **Filtered Graph Traversal** inside the HNSW layer:
- During graph expansion, non-matching nodes can still be traversed as routing hops to maintain navigable connectivity.
- Only points satisfying the filter predicate are considered as valid results.
- If a filter is extremely sparse (e.g. matching less than 0.5% of total points), Vectrust automatically performs an exhaustive fallback scan across the remaining points, ensuring that you always retrieve the expected `top_k` results without recall collapse.
