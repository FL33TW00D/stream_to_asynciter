use futures::{Stream, StreamExt};
use js_sys::{Object, Promise, Reflect, Symbol};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::future_to_promise;

struct AsyncIterator {
    object: Object,
    _async_iterator: Closure<dyn FnMut() -> Object>,
    _next: Closure<dyn FnMut() -> Promise>,
}

impl AsyncIterator {
    fn new<S>(stream: S) -> Self
    where
        S: Stream<Item = JsValue> + 'static,
    {
        let iterator = Object::new();

        let mut stream = Box::pin(stream);

        let next = Closure::new(move || {
            let next = stream.next();

            future_to_promise(async move {
                let output = Object::new();

                match next.await {
                    Some(value) => {
                        Reflect::set(&output, &JsValue::from("done"), &JsValue::from(false));
                        Reflect::set(&output, &JsValue::from("value"), &value);
                    }
                    None => {
                        Reflect::set(&output, &JsValue::from("done"), &JsValue::from(true));
                    }
                }

                Ok(output.into())
            })
        });

        Reflect::set(&iterator, &JsValue::from("next"), &next.as_ref());

        let object = Object::new();

        let async_iterator = Closure::new(move || iterator.clone());

        Reflect::set(&object, &Symbol::async_iterator(), &async_iterator.as_ref());

        Self {
            object,
            _async_iterator: async_iterator,
            _next: next,
        }
    }
}

fn main() {
    println!("Hello, world!");
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn stream_to_async_iterator() {
        let stream = futures::stream::iter(vec![1, 2, 3, 4, 5].into_iter().map(JsValue::from));
        let iterator = AsyncIterator::new(stream);

        let output = js_sys::Array::from(&iterator.object);

        assert_eq!(output.length(), 5);
    }
}
