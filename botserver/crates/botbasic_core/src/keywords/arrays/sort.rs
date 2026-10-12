use botbasic_types::UserSession;
use botbasic_types::BasicRuntime;
use log::debug;
use rhai::{Array, Dynamic, Engine};
use std::sync::Arc;

pub fn sort_keyword(_state: &Arc<dyn BasicRuntime>, _user: UserSession, engine: &mut Engine) {
    engine.register_fn("SORT", |arr: Array| -> Array { sort_array(arr, false) });

    engine.register_fn("sort", |arr: Array| -> Array { sort_array(arr, false) });

    engine.register_fn("SORT", |arr: Array, direction: &str| -> Array {
        let desc =
            direction.eq_ignore_ascii_case("DESC") || direction.eq_ignore_ascii_case("DESCENDING");
        sort_array(arr, desc)
    });

    engine.register_fn("sort", |arr: Array, direction: &str| -> Array {
        let desc =
            direction.eq_ignore_ascii_case("DESC") || direction.eq_ignore_ascii_case("DESCENDING");
        sort_array(arr, desc)
    });

    engine.register_fn("SORT_ASC", |arr: Array| -> Array { sort_array(arr, false) });

    engine.register_fn("sort_asc", |arr: Array| -> Array { sort_array(arr, false) });

    engine.register_fn("SORT_DESC", |arr: Array| -> Array { sort_array(arr, true) });

    engine.register_fn("sort_desc", |arr: Array| -> Array { sort_array(arr, true) });

    debug!("Registered SORT keyword");
}

fn sort_array(mut arr: Array, descending: bool) -> Array {
    arr.sort_by(|a, b| {
        let cmp = compare_dynamic(a, b);
        if descending {
            cmp.reverse()
        } else {
            cmp
        }
    });

    arr
}

fn compare_dynamic(a: &Dynamic, b: &Dynamic) -> std::cmp::Ordering {
    if let (Some(a_num), Some(b_num)) = (to_f64(a), to_f64(b)) {
        return a_num
            .partial_cmp(&b_num)
            .unwrap_or(std::cmp::Ordering::Equal);
    }

    a.to_string().cmp(&b.to_string())
}

fn to_f64(value: &Dynamic) -> Option<f64> {
    if value.is_int() {
        value.as_int().ok().map(|i| i as f64)
    } else if value.is_float() {
        value.as_float().ok()
    } else if value.is_string() {
        value
            .clone()
            .into_string()
            .ok()
            .and_then(|s| s.parse().ok())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rhai::{Array, Dynamic};

    #[test]
    fn test_sort_integers() {
        let arr: Array = vec![
            Dynamic::from_int(3),
            Dynamic::from_int(1),
            Dynamic::from_int(4),
            Dynamic::from_int(1),
            Dynamic::from_int(5),
        ];
        let sorted = sort_array(arr, false);
        assert_eq!(sorted[0].as_int().unwrap(), 1);
        assert_eq!(sorted[1].as_int().unwrap(), 1);
        assert_eq!(sorted[2].as_int().unwrap(), 3);
        assert_eq!(sorted[3].as_int().unwrap(), 4);
        assert_eq!(sorted[4].as_int().unwrap(), 5);
    }

    #[test]
    fn test_sort_strings() {
        let arr: Array = vec![
            Dynamic::from("banana"),
            Dynamic::from("apple"),
            Dynamic::from("cherry"),
        ];
        let sorted = sort_array(arr, false);
        assert_eq!(sorted[0].clone().into_string().unwrap(), "apple");
        assert_eq!(sorted[1].clone().into_string().unwrap(), "banana");
        assert_eq!(sorted[2].clone().into_string().unwrap(), "cherry");
    }

    #[test]
    fn test_sort_descending() {
        let arr: Array = vec![Dynamic::from_int(1), Dynamic::from_int(3), Dynamic::from_int(2)];
        let sorted = sort_array(arr, true);
        assert_eq!(sorted[0].as_int().unwrap(), 3);
        assert_eq!(sorted[1].as_int().unwrap(), 2);
        assert_eq!(sorted[2].as_int().unwrap(), 1);
    }

    #[test]
    fn test_compare_dynamic_numbers() {
        let a = Dynamic::from_int(5);
        let b = Dynamic::from_int(3);
        assert_eq!(compare_dynamic(&a, &b), std::cmp::Ordering::Greater);
    }

    #[test]
    fn test_compare_dynamic_strings() {
        let a = Dynamic::from("apple");
        let b = Dynamic::from("banana");
        assert_eq!(compare_dynamic(&a, &b), std::cmp::Ordering::Less);
    }
}
