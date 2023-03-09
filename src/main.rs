use futures_core::Stream;
use futures_util::StreamExt;
use js_sys::{Object, Promise, Reflect, Symbol};
use std::cell::RefCell;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::future_to_promise;

#[wasm_bindgen]
extern "C" {
    type IteratorResult;

    #[wasm_bindgen(method, setter)]
    fn set_done(this: &IteratorResult, value: bool);

    #[wasm_bindgen(method, setter)]
    fn set_value(this: &IteratorResult, value: &JsValue);
}

pub struct AsyncIterator {
    object: Object,
    _async_iterator: Closure<dyn FnMut() -> Object>,
    _next: Closure<dyn FnMut() -> Promise>,
}

impl AsyncIterator {
    pub fn new<S>(stream: S) -> Self
    where
        S: Stream<Item = JsValue> + 'static,
    {
        let iterator = Object::new();

        struct StreamBroadcaster {
            stream: Rc<RefCell<Pin<Box<dyn Stream<Item = JsValue>>>>>,
        }

        impl Clone for StreamBroadcaster {
            fn clone(&self) -> Self {
                Self {
                    stream: self.stream.clone(),
                }
            }
        }

        impl Stream for StreamBroadcaster {
            type Item = JsValue;

            fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
                let mut lock = self.stream.borrow_mut();
                lock.as_mut().poll_next(cx)
            }
        }

        let state = StreamBroadcaster {
            stream: Rc::new(RefCell::new(Box::pin(stream))),
        };

        let next = Closure::new(move || {
            let mut state = state.clone();

            future_to_promise(async move {
                let output = Object::new().unchecked_into::<IteratorResult>();

                match state.next().await {
                    Some(value) => {
                        output.set_done(false);
                        output.set_value(&value);
                    }
                    None => {
                        output.set_done(true);
                    }
                }

                Ok(output.into())
            })
        });

        Reflect::set(&iterator, &JsValue::from("next"), &next.as_ref()).unwrap();

        let object = Object::new();

        let async_iterator = Closure::new(move || -> Object { iterator.clone() });

        Reflect::set(&object, &Symbol::async_iterator(), &async_iterator.as_ref()).unwrap();

        Self {
            object,
            _async_iterator: async_iterator,
            _next: next,
        }
    }
}

impl AsRef<JsValue> for AsyncIterator {
    fn as_ref(&self) -> &JsValue {
        &self.object
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

        let test = iterator.as_ref();

        let async_iter = js_sys::Function::new_with_args(
            "test",
            "return async function*(test) { 
                for await (let value of test) {
                    yield value;
                }
           
        }()",
        )
        .call1(&JsValue::NULL, &test)
        .unwrap();

        println!("{:?}", async_iter);
    }
}
