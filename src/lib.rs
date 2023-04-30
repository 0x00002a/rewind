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
pub fn atom<T, Undo: FnOnce(T)>(value: T, undo: Undo) -> atom::ValAtom<T, Undo> {
    atom::ValAtom::new(value, undo)
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
