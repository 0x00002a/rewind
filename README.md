# Rewind
_Its rewind time babeee_


![License: GPLv3](https://img.shields.io/github/license/0x00002a/rewind?style=flat-square)
![GitHub Workflow Status](https://img.shields.io/github/actions/workflow/status/0x00002a/rewind/build.yml?style=flat-square)

This crate contains utilities to help with developing APIs with [strong exception
guarantees](https://en.wikipedia.org/wiki/Exception_safety). Basically, if the function
fails in some way then it should be like the function was never called.

In languages such as C# and Java this can be done with `finally` blocks. Rust currently however
has no way to "catch" a `?` return outside of putting it in a lambda, and anyway sometimes you want
finer grain control than entire blocks. For example, when you have multiple operations that depend on
the previous one block statements are quite unwieldy:

```rust
#[derive(Default)]
struct Stack<T> {
    els: Vec<T>,
}
impl <T> Stack<T> {
    pub fn pop(&mut self) -> Result<T, ()> {
        self.els.pop().ok_or(())
    }
    pub fn get(&self, index: usize) -> Result<&T, ()> {
        self.els.get(index).ok_or(())
    }
    pub fn push(&mut self, el: T) {
        self.els.push(el);
    }
}
fn may_fail() -> Result<(), ()> { Err(()) }

let mut s = Stack::<i32>::default();
let result = (|| {
    s.push(4);
    s.push(5);
    let value = s.pop()?;
    may_fail()?;
    println!("{}", value);
    Ok::<(), ()>(())
})();
if result.is_err() {
    s.push(4);
}
assert_eq!(s.els, vec![4, 4]); // uh oh
```

As indicated by the comment, the above code will push `4` to the stack twice. Although the example is simple, this
mistake is quite easy to make in code where the side effects are complex. Now lets look at this code using `rewind`:


```rust
use rewind::Atom;
#[derive(Default)]
struct Stack<T> {
    els: Vec<T>,
}
impl<T> Stack<T> {
    pub fn pop(&mut self) -> Result<T, ()> {
        self.els.pop().ok_or(())
    }

    pub fn push(&mut self, el: T) {
        self.els.push(el);
    }
}
fn may_fail() -> Result<(), ()> {
    Err(())
}

let mut s = rewind::encase(Stack::<i32>::default());
let result = (|| {
    s.push(4);
    s.push(5);
    let value = s.peel_mut(
        |s| s.pop(),
        |s, v| {
            if let Ok(v) = v {
                s.push(v);
            }
        },
    );
    may_fail()?;
    println!("{}", value.decay()?);
    Ok::<(), ()>(())
})();
assert!(result.is_err());
assert_eq!(s.els, vec![4, 5]);
```

The rewind version uses [`peel_mut`](rewind::atom::Encased::peel_mut) in order to
define the `pop` with an undo. The [`decay`](rewind::Atom::decay) call would
cause the undo to be forgotten about, but since `may_fail` will cause an early
return before that point, the undo action is called and the pop is undone.


For values that are cheap to copy, [`own_id`](rewind::own_id) can be used to "peel off" a field and modify it. For example:

```rust
use rewind::Atom;
struct Person {
    name: String,
    age: u8,
}
impl Person {
    fn set_name_and_age(&mut self, name: String, age: u8) {
        let mut name_am = rewind::own_id(self.name.clone());
        *name_am = name;
        if age > 200 {
            return;
        }
        self.age = age;
        self.name = name_am.decay();
    }
}
let mut p = Person { name: "Sarah".to_owned(), age: 43 };

p.set_name_and_age("Sasha".to_owned(), 201);
assert_eq!(&p.name, "Sarah");
assert_eq!(p.age, 43);

p.set_name_and_age("Sasha".to_owned(), 44);
assert_eq!(&p.name, "Sasha");
assert_eq!(p.age, 44);
```

Now this is of course quite a contrived example, the API for `Person` is very
strange and the simplest way to obey the contract would be to just move the
assignment to `name` to after the validation. It does however show how
[`own`](rewind::own) can be used for modifying struct values in an error safe
way.
