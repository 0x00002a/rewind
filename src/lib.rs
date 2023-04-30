#![doc = include_str!("../README.md")]

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
/// items.undo();
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
/// use rewind::atom::Atom;
/// let mut items = vec!["a", "b"];
/// let mut items = rewind::own(items, |v| v);
/// items.clear();
/// assert_eq!(items.len(), 0);
/// items.undo();
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
    use super::*;

    #[test]
    fn t1() {
        let mut items = vec![1, 2, 3];
        let mut items = encase(items);
        items.peel_mut(
            |i| i.push(3),
            |i, _| {
                i.pop();
            },
        );
        items.peel_mut(
            |i| i.push(5),
            |i, _| {
                i.pop();
            },
        );
        let v = items.peel_mut(
            |i| i.pop(),
            |i, v| {
                if let Some(v) = v {
                    i.push(v);
                }
            },
        );
        assert_eq!(*v, Some(5));
    }
}
