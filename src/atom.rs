pub struct Atom<Undo: FnOnce()> {
    undo: Option<Undo>,
}

impl<Undo: FnOnce()> Atom<Undo> {
    pub fn new(undo: Undo) -> Self {
        Self { undo: Some(undo) }
    }
    pub fn cancel(mut self) {
        self.undo.take();
    }
}
impl<Undo: FnOnce()> Drop for Atom<Undo> {
    fn drop(&mut self) {
        if let Some(undo) = self.undo.take() {
            undo();
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
            let atom = Atom::new(|| {
                scoped = 0;
            });
            drop(atom);
        }
        assert_eq!(scoped, 0);
    }

    #[test]
    fn cancelling_atom_stops_it_running_on_drop() {
        let mut scoped = 12;
        {
            let atom = Atom::new(|| {
                scoped = 0;
            });
            atom.cancel();
        }
        assert_eq!(scoped, 12);
    }
}
