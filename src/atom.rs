/// Carries a value with an undo action. If dropped without a call to `Atom::cancel` the undo
/// action is called. See `rewind::atom` for usage examples
pub struct ValAtom<T, Undo: FnOnce(T)> {
    val: Option<T>,
    undo: Option<Undo>,
}

impl<T, Undo: FnOnce(T)> ValAtom<T, Undo> {
    pub fn new(val: T, undo: Undo) -> Self {
        Self {
            val: Some(val),
            undo: Some(undo),
        }
    }
}

impl<T, Undo: FnOnce(T)> Atom for ValAtom<T, Undo> {
    fn undo(&mut self) {
        if let Some(undo) = self.undo.take() {
            undo(self.val.take().unwrap());
        }
    }
    fn cancel(&mut self) {
        self.undo.take();
        self.val.take();
    }
}

impl<T, Undo: FnOnce(T)> Drop for ValAtom<T, Undo> {
    fn drop(&mut self) {
        self.undo();
    }
}

pub trait Atom {
    fn undo(&mut self);
    fn cancel(&mut self);
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn atom_runs_on_drop_if_uncancelled() {
        let mut scoped = 12;
        {
            let atom = ValAtom::new(&mut scoped, |v| {
                *v = 0;
            });
            drop(atom);
        }
        assert_eq!(scoped, 0);
    }

    #[test]
    fn cancelling_atom_stops_it_running_on_drop() {
        let mut scoped = 12;
        {
            let mut atom = ValAtom::new(&mut scoped, |s| {
                *s = 13;
            });
            atom.cancel();
        }
        assert_eq!(scoped, 12);
    }
}
