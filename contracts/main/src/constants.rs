pub const HASH_SIZE: usize = 32;

#[repr(isize)]
pub enum Error {
    InvalidOutputTypeHash = -6,
    InvalidWitness = -11,
    Panic = -42,
    OutOfMemory = 255,
}
