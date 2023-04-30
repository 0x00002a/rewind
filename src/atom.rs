pub struct Atom<T, Undo: FnOnce(T)> {
    val: Option<T>,
    undo: Option<Undo>,
}

impl<T, Undo: FnOnce(T)> Atom<T, Undo> {
    pub fn new(val: T, undo: Undo) -> Self {
        Self {
            val: Some(val),
            undo: Some(undo),
        }
    }
    pub fn cancel(&mut self) {
        self.undo.take();
        self.val.take();
    }
}
impl<T, Undo: FnOnce(T)> Drop for Atom<T, Undo> {
    fn drop(&mut self) {
        if let Some(undo) = self.undo.take() {
            undo(self.val.take().unwrap());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atom_runs_on_drop_if_uncancelled() {
        let mut scoped = 12;
        {
            let atom = Atom::new(&mut scoped, |v| {
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
            let mut atom = Atom::new(&mut scoped, |s| {
                *s = 13;
            });
            atom.cancel();
        }
        assert_eq!(scoped, 12);
    }
}
