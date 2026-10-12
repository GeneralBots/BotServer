use botbasic_types::UserSession;
use botbasic_types::BasicRuntime;
use log::debug;
use rhai::{Array, Dynamic, Engine};
use std::sync::Arc;

pub fn push_keyword(_state: &Arc<dyn BasicRuntime>, _user: UserSession, engine: &mut Engine) {
    engine.register_fn("PUSH", |mut arr: Array, value: Dynamic| -> Array {
        arr.push(value);
        arr
    });

    engine.register_fn("push", |mut arr: Array, value: Dynamic| -> Array {
        arr.push(value);
        arr
    });

    engine.register_fn("ARRAY_PUSH", |mut arr: Array, value: Dynamic| -> Array {
        arr.push(value);
        arr
    });

    engine.register_fn("APPEND", |mut arr: Array, value: Dynamic| -> Array {
        arr.push(value);
        arr
    });

    engine.register_fn("append", |mut arr: Array, value: Dynamic| -> Array {
        arr.push(value);
        arr
    });

    debug!("Registered PUSH keyword");
}

pub fn pop_keyword(_state: &Arc<dyn BasicRuntime>, _user: UserSession, engine: &mut Engine) {
    engine.register_fn("POP", |mut arr: Array| -> Dynamic {
        arr.pop().unwrap_or(Dynamic::UNIT)
    });

    engine.register_fn("pop", |mut arr: Array| -> Dynamic {
        arr.pop().unwrap_or(Dynamic::UNIT)
    });

    engine.register_fn("ARRAY_POP", |mut arr: Array| -> Dynamic {
        arr.pop().unwrap_or(Dynamic::UNIT)
    });

    debug!("Registered POP keyword");
}

pub fn shift_keyword(_state: &Arc<dyn BasicRuntime>, _user: UserSession, engine: &mut Engine) {
    engine.register_fn("SHIFT", |mut arr: Array| -> Dynamic {
        if arr.is_empty() {
            Dynamic::UNIT
        } else {
            arr.remove(0)
        }
    });

    engine.register_fn("shift", |mut arr: Array| -> Dynamic {
        if arr.is_empty() {
            Dynamic::UNIT
        } else {
            arr.remove(0)
        }
    });

    engine.register_fn("ARRAY_SHIFT", |mut arr: Array| -> Dynamic {
        if arr.is_empty() {
            Dynamic::UNIT
        } else {
            arr.remove(0)
        }
    });

    debug!("Registered SHIFT keyword");
}

pub fn unshift_keyword(_state: &Arc<dyn BasicRuntime>, _user: UserSession, engine: &mut Engine) {
    engine.register_fn("UNSHIFT", |mut arr: Array, value: Dynamic| -> Array {
        arr.insert(0, value);
        arr
    });

    engine.register_fn("unshift", |mut arr: Array, value: Dynamic| -> Array {
        arr.insert(0, value);
        arr
    });

    engine.register_fn("PREPEND", |mut arr: Array, value: Dynamic| -> Array {
        arr.insert(0, value);
        arr
    });

    engine.register_fn("prepend", |mut arr: Array, value: Dynamic| -> Array {
        arr.insert(0, value);
        arr
    });

    debug!("Registered UNSHIFT keyword");
}

#[cfg(test)]
mod tests {
    use rhai::{Array, Dynamic};

    #[test]
    fn test_push() {
        let mut arr: Array = vec![Dynamic::from_int(1), Dynamic::from_int(2)];
        arr.push(Dynamic::from_int(3));
        assert_eq!(arr.len(), 3);
        assert_eq!(arr[2].as_int().unwrap_or(0), 3);
    }

    #[test]
    fn test_pop() {
        let mut arr: Array = vec![Dynamic::from_int(1), Dynamic::from_int(2), Dynamic::from_int(3)];
        let popped = arr.pop();
        assert_eq!(arr.len(), 2);
        assert_eq!(popped.and_then(|v| v.as_int().ok()).unwrap_or(0), 3);
    }

    #[test]
    fn test_pop_empty() {
        let mut arr: Array = vec![];
        let popped = arr.pop();
        assert!(popped.is_none());
    }

    #[test]
    fn test_shift() {
        let mut arr: Array = vec![Dynamic::from_int(1), Dynamic::from_int(2), Dynamic::from_int(3)];
        let shifted = arr.remove(0);
        assert_eq!(arr.len(), 2);
        assert_eq!(shifted.as_int().unwrap_or(0), 1);
        assert_eq!(arr[0].as_int().unwrap_or(0), 2);
    }

    #[test]
    fn test_unshift() {
        let mut arr: Array = vec![Dynamic::from_int(2), Dynamic::from_int(3)];
        arr.insert(0, Dynamic::from_int(1));
        assert_eq!(arr.len(), 3);
        assert_eq!(arr[0].as_int().unwrap_or(0), 1);
    }
}
