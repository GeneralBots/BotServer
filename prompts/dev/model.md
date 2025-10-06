
Create a Rust data model for database storage with optimal size and performance characteristics. Follow these specifications:

**REQUIREMENTS:**
1. Use appropriate integer types (i32, i16, i8, etc.) based on expected value ranges
2. Use `Option<T>` for nullable fields to avoid memory overhead
3. Use `Vec<u8>` for binary data instead of strings when appropriate
4. Prefer enum representations as integers rather than strings
5. Use `chrono::DateTime<Utc>` for timestamps
6. Use `uuid::Uuid` for unique identifiers
7. Implement necessary traits: `Debug`, `Clone`, `Serialize`, `Deserialize`, `FromRow`
8. Include validation where appropriate
9. Consider database index strategy in field design

**CONTEXT:**
- Database: PostgreSQL/SQLx compatible
- Serialization: Serde for JSON
- ORM: SQLx for database operations

**OUTPUT FORMAT:**
Provide the complete Rust struct with:
- Struct definition with fields
- Enum definitions with integer representations
- Conversion implementations
- Basic validation if needed

**EXAMPLE REFERENCE:**
```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Status {
    Pending = 0,
    Active = 1,
    Inactive = 2,
}

impl Status {
    pub fn from_i16(value: i16) -> Option<Self> {
        match value {
            0 => Some(Self::Pending),
            1 => Some(Self::Active),
            2 => Some(Self::Inactive),
            _ => None,
        }
    }
}

#[derive(Debug, FromRow, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub status: i16, // Using i16 for enum storage
    pub email: String,
    pub age: Option<i16>, // Nullable small integer
    pub metadata: Vec<u8>, // Binary data for flexibility
    pub created_at: DateTime<Utc>,
}
```

Generate a similar model for: [YOUR DOMAIN HERE]
```

## Specialized Variants

### For High-Performance Applications
```
Add these additional requirements:
- Use `#[repr(u8)]` for enums to ensure minimal size
- Consider `Box<str>` instead of `String` for reduced heap overhead
- Use `arrayvec::ArrayString` for fixed-size short strings
- Implement `PartialEq` and `Eq` for hash-based operations
- Include `#[derive(Default)]` where appropriate
```

### For Embedded/Memory-Constrained Systems
```
Add these constraints:
- Prefer `i16` over `i32` where possible
- Use `u32` instead of `Uuid` if sequential IDs are acceptable
- Consider `bitflags` for multiple boolean flags in single byte
- Use `smol_str::SmolStr` for string optimization
- Avoid `Vec` in favor of arrays with capacity limits
```

### For Time-Series Data
```
Add time-series specific optimizations:
- Use `i64` for timestamps as nanoseconds since epoch
- Use `f32` instead of `f64` for measurements where precision allows
- Consider `ordered_float::OrderedFloat` for floating-point comparisons
- Use `#[serde(with = "chrono::serde::ts_seconds")]` for compact serialization
