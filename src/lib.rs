#![doc = include_str!("../README.md")]

extern crate self as rewind;

pub mod atom;

pub use atom::Atom;

/// Identity
///
/// Here to help with functions such as [`rewind::own`]
///
/// ```
/// use rewind::Atom;
/// let mut items = rewind::own(vec![1, 2, 3], rewind::id);
/// items.clear();
/// let items = items.undo();
/// assert_eq!(items.len(), 3);
/// ```
pub fn id<T>(v: T) -> T {
    v
}

/// Create an undo operation with stored data
///
pub fn simple<T, R, Undo: FnOnce(T) -> R>(value: T, undo: Undo) -> atom::Simple<T, R, Undo> {
    atom::Simple::new(value, undo)
}

/// Provides a way around rust's ownership requirements.
///
/// E.g. the following code does not compile:
/// ```compile_fail
/// let mut items = vec!["a", "b"];
/// let mut op = rewind::atom(items.clone(), |v| items = v);
/// items.clear(); // boom
/// drop(items);
/// ```
/// Instead we need to let the atom keep the ownership, thats where [`own`] comes in:
/// ```
/// # use rewind::atom::Atom;
/// let items = vec!["a", "b"];
/// let mut items = rewind::own(items, |v| v);
/// items.clear();
/// assert_eq!(items.len(), 0);
/// let items = items.undo();
/// assert_eq!(items.len(), 2);
/// ```
///
pub fn own<T: Clone, Undo: FnOnce(T) -> T>(value: T, undo: Undo) -> atom::Owning<T, Undo> {
    atom::Owning::new(value, undo)
}

pub fn encase<S>(s: S) -> atom::Encased<S> {
    atom::Encased::new(s)
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::*;

    #[test]
    fn isomorphic_attr() {
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
        assert_eq!(s.els, vec![4, 5]); // uh oh
    }
    #[test]
    fn encasing_cannot_leak_abstraction_and_cause_panic_due_to_multiple_borrows() {
        let mut items = encase(vec![1, 2, 3]);
        let b1 = items.peel_mut(|i| i.push(4), |i, _| i.pop());
        let b2 = items.peel_mut(|i| i.push(5), |i, _| i.pop());
    }
    #[test]
    fn peeling_mutably_allows_reversing_a_mutable_operation() {
        let mut items = encase(vec![1, 2, 3]);
        let v = items.peel_mut(
            |i| i.pop(),
            |i, v| {
                if let Some(v) = v {
                    i.push(v);
                }
            },
        );
        assert_eq!(*v, Some(3));
        assert_eq!(items.len(), 2);
        v.undo();
        assert_eq!(items.len(), 3);
    }
}
