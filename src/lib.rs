pub mod atom;
pub mod stack;

pub fn using<T, E, R>(
    mut value: T,
    act: impl FnOnce(&mut T) -> std::result::Result<R, E>,
    undo: impl FnOnce(&mut T),
) -> std::result::Result<R, E> {
    match act(&mut value) {
        Ok(v) => Ok(v),
        Err(e) => {
            undo(&mut value);
            Err(e)
        }
    }
}

/// Create an undo operation with stored data
///
pub fn atom<T, Undo: FnOnce(T)>(value: T, undo: Undo) -> atom::ValAtom<T, (), Undo> {
    atom::ValAtom::new(value, undo)
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
pub fn own<T: Clone, Undo: FnOnce(T) -> T>(value: T, undo: Undo) -> atom::StoreAtom<T, Undo> {
    atom::StoreAtom::new(value, undo)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn atom_1() {
        let mut vec = Vec::new();
        using(
            &mut vec,
            |v| {
                v.push(0);
                Ok::<(), ()>(())
            },
            |v| {
                v.remove(0);
            },
        )
        .unwrap();
    }
}
