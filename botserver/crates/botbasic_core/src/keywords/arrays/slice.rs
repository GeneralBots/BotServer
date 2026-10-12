use botbasic_types::UserSession;
use botbasic_types::BasicRuntime;
use log::debug;
use rhai::{Array, Dynamic, Engine};
use std::sync::Arc;

pub fn slice_keyword(_state: &Arc<dyn BasicRuntime>, _user: UserSession, engine: &mut Engine) {
    engine.register_fn("SLICE", |arr: Array, start: i64| -> Array {
        slice_array(&arr, start, None)
    });

    engine.register_fn("slice", |arr: Array, start: i64| -> Array {
        slice_array(&arr, start, None)
    });

    engine.register_fn("SLICE", |arr: Array, start: i64, end: i64| -> Array {
        slice_array(&arr, start, Some(end))
    });

    engine.register_fn("slice", |arr: Array, start: i64, end: i64| -> Array {
        slice_array(&arr, start, Some(end))
    });

    engine.register_fn("SUBARRAY", |arr: Array, start: i64, end: i64| -> Array {
        slice_array(&arr, start, Some(end))
    });

    engine.register_fn(
        "MID_ARRAY",
        |arr: Array, start: i64, length: i64| -> Array {
            let end = start + length;
            slice_array(&arr, start, Some(end))
        },
    );

    engine.register_fn("TAKE", |arr: Array, count: i64| -> Array {
        slice_array(&arr, 0, Some(count))
    });

    engine.register_fn("take", |arr: Array, count: i64| -> Array {
        slice_array(&arr, 0, Some(count))
    });

    engine.register_fn("DROP", |arr: Array, count: i64| -> Array {
        slice_array(&arr, count, None)
    });

    engine.register_fn("drop", |arr: Array, count: i64| -> Array {
        slice_array(&arr, count, None)
    });

    engine.register_fn("HEAD", |arr: Array| -> Dynamic {
        arr.first().cloned().unwrap_or(Dynamic::UNIT)
    });

    engine.register_fn("head", |arr: Array| -> Dynamic {
        arr.first().cloned().unwrap_or(Dynamic::UNIT)
    });

    engine.register_fn("TAIL", |arr: Array| -> Array {
        if arr.len() <= 1 {
            Array::new()
        } else {
            arr[1..].to_vec()
        }
    });

    engine.register_fn("tail", |arr: Array| -> Array {
        if arr.len() <= 1 {
            Array::new()
        } else {
            arr[1..].to_vec()
        }
    });

    engine.register_fn("INIT", |arr: Array| -> Array {
        if arr.is_empty() {
            Array::new()
        } else {
            arr[..arr.len() - 1].to_vec()
        }
    });

    engine.register_fn("LAST", |arr: Array| -> Dynamic {
        arr.last().cloned().unwrap_or(Dynamic::UNIT)
    });

    debug!("Registered SLICE keyword");
}

fn slice_array(arr: &Array, start: i64, end: Option<i64>) -> Array {
    let len = arr.len() as i64;

    let start_idx = if start < 0 {
        (len + start).max(0) as usize
    } else {
        (start as usize).min(arr.len())
    };

    let end_idx = match end {
        Some(e) => {
            if e < 0 {
                (len + e).max(0) as usize
            } else {
                (e as usize).min(arr.len())
            }
        }
        None => arr.len(),
    };

    if start_idx >= end_idx || start_idx >= arr.len() {
        return Array::new();
    }

    arr[start_idx..end_idx].to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rhai::{Array, Dynamic};

    fn make_test_array() -> Array {
        vec![
            Dynamic::from_int(1),
            Dynamic::from_int(2),
            Dynamic::from_int(3),
            Dynamic::from_int(4),
            Dynamic::from_int(5),
        ]
    }

    #[test]
    fn test_slice_from_start() {
        let arr = make_test_array();
        let result = slice_array(&arr, 2, None);
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].as_int().unwrap(), 3);
    }

    #[test]
    fn test_slice_with_end() {
        let arr = make_test_array();
        let result = slice_array(&arr, 1, Some(3));
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].as_int().unwrap(), 2);
        assert_eq!(result[1].as_int().unwrap(), 3);
    }

    #[test]
    fn test_slice_negative_start() {
        let arr = make_test_array();
        let result = slice_array(&arr, -2, None);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].as_int().unwrap(), 4);
        assert_eq!(result[1].as_int().unwrap(), 5);
    }

    #[test]
    fn test_slice_negative_end() {
        let arr = make_test_array();
        let result = slice_array(&arr, 0, Some(-2));
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].as_int().unwrap(), 1);
        assert_eq!(result[2].as_int().unwrap(), 3);
    }

    #[test]
    fn test_slice_out_of_bounds() {
        let arr = make_test_array();
        let result = slice_array(&arr, 10, None);
        assert!(result.is_empty());
    }

    #[test]
    fn test_slice_empty_range() {
        let arr = make_test_array();
        let result = slice_array(&arr, 3, Some(2));
        assert!(result.is_empty());
    }
}
