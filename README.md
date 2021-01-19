# Usage example:

Create a new rust crate and make sure to specify crate-type to be "dylib".
Also add `rutie`, `rutie-serde`, `serde` and possibly `serde_derive` as a dependency.

```toml
[package]
edition = "2018"

[lib]
crate-type = ["dylib"]
name = "ruby_rust_demo"

[dependencies]
rutie = "0.8"
rutie-serde = "0.2"
serde = "1.0"
serde_derive = "1.0"
```

The usage is very similar to how you would use `rutie` on it's own, but instead of calling
`rutie_methods!` macro, you call `rutie_serde_methods!`.
This macro takes care of deserializing arguments and serializing return values.
It also captures all panics inside those methods and raises them as an exception in ruby.

```rust
use rutie::{class, Class, Object};
use rutie_serde::{ruby_class, rutie_serde_methods};
use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct User {
    pub name: String,
    pub id: u64,
}

class!(HelloWorld);
rutie_serde_methods!(
    HelloWorld,
    _itself,
    ruby_class!(Exception),
    fn hello(name: String) -> String {
        format!("Hello {}", name)
    }
    fn hello_user(user: User) -> String {
        format!("Hello {:?}", user)
    }
);

#[allow(non_snake_case)]
#[no_mangle]
pub extern "C" fn Init_ruby_rust_demo() {
    let mut class = Class::new("RubyRustDemo", None);
    class.define(|itself| itself.def_self("hello", hello));
    class.define(|itself| itself.def_self("hello_user", hello_user));
}
```
