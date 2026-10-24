/// Comandi dall'esterno verso il motore.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    HaltTx,
    Reply { decode_id: u64 },
}
